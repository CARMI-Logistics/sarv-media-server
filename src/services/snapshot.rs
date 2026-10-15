//! Captura de un frame (JPEG) de una cámara vía ffmpeg (sub-plan A, paso 2).
//!
//! ffmpeg lee el RTSP interno de MediaMTX reutilizando el JWT del request
//! (`?jwt=`), así la lectura queda sujeta al mismo control de acceso que HLS.
//! La concurrencia se acota con un semáforo y el proceso hijo muere si vence
//! el timeout (`kill_on_drop`), para no dejar ffmpeg huérfanos.

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tracing::warn;

/// Ancho mínimo/máximo/por defecto aceptado por el Contrato.
pub const MIN_WIDTH: u32 = 160;
pub const MAX_WIDTH: u32 = 1920;
pub const DEFAULT_WIDTH: u32 = 1280;

/// Tiempo máximo para obtener un frame (Contrato: 504 si se excede).
pub const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(10);

/// Marca de inicio de todo JPEG (SOI).
const JPEG_SOI: [u8; 2] = [0xFF, 0xD8];

#[derive(Debug, PartialEq, Eq)]
pub enum SnapshotError {
    /// No hubo frame dentro del tiempo límite (incluye la espera por el semáforo).
    Timeout,
    /// ffmpeg no arrancó, falló o no produjo un JPEG.
    Decoder,
}

pub struct SnapshotService {
    ffmpeg_bin: String,
    rtsp_base_url: String,
    limiter: Semaphore,
    timeout: Duration,
}

impl SnapshotService {
    pub fn new(
        ffmpeg_bin: impl Into<String>,
        rtsp_base_url: impl Into<String>,
        max_concurrent: usize,
        timeout: Duration,
    ) -> Self {
        Self {
            ffmpeg_bin: ffmpeg_bin.into(),
            rtsp_base_url: rtsp_base_url.into().trim_end_matches('/').to_string(),
            limiter: Semaphore::new(max_concurrent.max(1)),
            timeout,
        }
    }

    /// Devuelve un JPEG del encuadre completo de `path`, escalado a `width`
    /// conservando la proporción.
    pub async fn capture(&self, path: &str, jwt: &str, width: u32) -> Result<Vec<u8>, SnapshotError> {
        timeout(self.timeout, async {
            let _permit = self.limiter.acquire().await.map_err(|_| SnapshotError::Decoder)?;
            self.run_ffmpeg(path, jwt, width).await
        })
        .await
        .map_err(|_| SnapshotError::Timeout)?
    }

    async fn run_ffmpeg(&self, path: &str, jwt: &str, width: u32) -> Result<Vec<u8>, SnapshotError> {
        // La URL lleva el JWT: no se loguea ni se propaga en errores.
        let url = format!("{}/{}?jwt={}", self.rtsp_base_url, path, jwt);
        let output = Command::new(&self.ffmpeg_bin)
            .args(["-nostdin", "-hide_banner", "-loglevel", "error"])
            .args(["-protocol_whitelist", "rtsp,tcp,rtp,udp"])
            .args(["-rtsp_transport", "tcp", "-i", &url])
            .args(["-frames:v", "1", "-vf", &format!("scale={width}:-2")])
            .args(["-f", "image2pipe", "-c:v", "mjpeg", "-q:v", "3", "pipe:1"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| {
                warn!("no se pudo ejecutar ffmpeg: {}", e);
                SnapshotError::Decoder
            })?;

        if !output.status.success() || !output.stdout.starts_with(&JPEG_SOI) {
            warn!("ffmpeg no produjo un JPEG para '{}' (status {})", path, output.status);
            return Err(SnapshotError::Decoder);
        }
        Ok(output.stdout)
    }
}

/// Interpreta el parámetro `width`: ausente → default; fuera de rango o no
/// numérico → `None` (el handler responde 422).
pub fn parse_width(raw: Option<&str>) -> Option<u32> {
    match raw {
        None => Some(DEFAULT_WIDTH),
        Some(s) => s
            .parse::<u32>()
            .ok()
            .filter(|w| (MIN_WIDTH..=MAX_WIDTH).contains(w)),
    }
}

/// Soporte de tests: un `ffmpeg` falso (script de shell) en un directorio temporal.
#[cfg(test)]
pub(crate) mod testing {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::Command;

    use uuid::Uuid;

    /// Salida mínima que `capture` acepta como JPEG (SOI + EOI).
    pub const FAKE_JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xD9];

    pub struct FakeFfmpeg {
        dir: PathBuf,
        bin: PathBuf,
    }

    impl FakeFfmpeg {
        /// Crea un ejecutable con el cuerpo de shell dado. El script puede usar
        /// `$DIR` (directorio temporal) para dejar evidencia de su ejecución.
        pub fn new(body: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("fake-ffmpeg-{}", Uuid::new_v4()));
            fs::create_dir_all(&dir).expect("crear directorio temporal");
            let bin = dir.join("ffmpeg");
            let script = format!(
                "#!/bin/sh\n[ \"$1\" = --warmup ] && exit 0\nDIR='{}'\n{}\n",
                dir.display(),
                body
            );
            fs::write(&bin, script).expect("escribir script");
            fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");
            // La primera ejecución de un binario recién creado puede tardar
            // (p.ej. el escaneo de macOS) y falsearía los tests con timeout corto:
            // se absorbe aquí, antes de que el test empiece a medir.
            let warmed = Command::new(&bin).arg("--warmup").status().expect("ejecutar script");
            assert!(warmed.success(), "el ffmpeg falso no arrancó");
            Self { dir, bin }
        }

        /// ffmpeg falso que imprime un JPEG válido y guarda sus argumentos.
        pub fn succeeding() -> Self {
            Self::new("printf '%s\\n' \"$@\" > \"$DIR/args\"\nprintf '\\377\\330\\377\\331'")
        }

        pub fn bin(&self) -> String {
            self.bin.display().to_string()
        }

        pub fn path(&self, name: &str) -> PathBuf {
            self.dir.join(name)
        }

        /// Argumentos con los que se invocó (una línea por argumento).
        pub fn recorded_args(&self) -> Vec<String> {
            fs::read_to_string(self.path("args"))
                .expect("el ffmpeg falso no registró argumentos")
                .lines()
                .map(str::to_string)
                .collect()
        }
    }

    impl Drop for FakeFfmpeg {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{FakeFfmpeg, FAKE_JPEG};
    use super::*;

    /// Holgado: los casos que terminan solos no deben depender de la carga del host.
    const AMPLE_TIMEOUT: Duration = Duration::from_secs(5);
    /// Corto: solo para los casos que provocan el timeout a propósito.
    const SHORT_TIMEOUT: Duration = Duration::from_millis(300);

    fn service_with(ffmpeg: &FakeFfmpeg, max_concurrent: usize, timeout: Duration) -> SnapshotService {
        SnapshotService::new(ffmpeg.bin(), "rtsp://mediamtx:8554/", max_concurrent, timeout)
    }

    fn service(ffmpeg: &FakeFfmpeg) -> SnapshotService {
        service_with(ffmpeg, 1, AMPLE_TIMEOUT)
    }

    #[test]
    fn parse_width_defaults_when_absent() {
        assert_eq!(parse_width(None), Some(DEFAULT_WIDTH));
    }

    #[test]
    fn parse_width_accepts_bounds() {
        assert_eq!(parse_width(Some("160")), Some(160));
        assert_eq!(parse_width(Some("1920")), Some(1920));
    }

    #[test]
    fn parse_width_rejects_out_of_range_and_non_numeric() {
        for raw in ["159", "1921", "0", "-5", "abc", "", "640.5"] {
            assert_eq!(parse_width(Some(raw)), None, "{raw:?} debería ser inválido");
        }
    }

    #[tokio::test]
    async fn capture_returns_jpeg_and_builds_rtsp_url_with_jwt() {
        let ffmpeg = FakeFfmpeg::succeeding();
        let jpeg = service(&ffmpeg).capture("cam-a", "tok.en", 640).await.unwrap();
        assert_eq!(jpeg, FAKE_JPEG);

        let args = ffmpeg.recorded_args();
        assert!(args.contains(&"rtsp://mediamtx:8554/cam-a?jwt=tok.en".to_string()));
        assert!(args.contains(&"scale=640:-2".to_string()));
        assert!(args.contains(&"-nostdin".to_string()));
        let whitelist = args.iter().position(|a| a == "-protocol_whitelist").unwrap();
        assert_eq!(args[whitelist + 1], "rtsp,tcp,rtp,udp");
    }

    #[tokio::test]
    async fn capture_maps_ffmpeg_failure_to_decoder_error() {
        let ffmpeg = FakeFfmpeg::new("exit 1");
        let result = service(&ffmpeg).capture("cam-a", "t", 640).await;
        assert_eq!(result, Err(SnapshotError::Decoder));
    }

    #[tokio::test]
    async fn capture_rejects_output_that_is_not_a_jpeg() {
        let ffmpeg = FakeFfmpeg::new("printf 'no soy un jpeg'");
        let result = service(&ffmpeg).capture("cam-a", "t", 640).await;
        assert_eq!(result, Err(SnapshotError::Decoder));
    }

    #[tokio::test]
    async fn capture_reports_decoder_error_when_binary_is_missing() {
        let service = SnapshotService::new("/no/existe/ffmpeg", "rtsp://x", 1, SHORT_TIMEOUT);
        assert_eq!(service.capture("cam-a", "t", 640).await, Err(SnapshotError::Decoder));
    }

    #[tokio::test]
    async fn capture_times_out_and_kills_the_child_process() {
        let ffmpeg = FakeFfmpeg::new("echo $$ > \"$DIR/pid\"\nexec sleep 30");
        // Margen suficiente para que el script alcance a escribir su pid antes del corte.
        let result = service_with(&ffmpeg, 1, Duration::from_secs(1)).capture("cam-a", "t", 640).await;
        assert_eq!(result, Err(SnapshotError::Timeout));

        let pid = std::fs::read_to_string(ffmpeg.path("pid")).unwrap().trim().to_string();
        assert!(process_is_gone(&pid).await, "ffmpeg {pid} quedó huérfano tras el timeout");
    }

    #[tokio::test]
    async fn capture_waiting_on_the_semaphore_counts_toward_the_timeout() {
        let ffmpeg = FakeFfmpeg::new("exec sleep 30");
        let service = std::sync::Arc::new(service_with(&ffmpeg, 1, SHORT_TIMEOUT));
        let holder = {
            let service = service.clone();
            tokio::spawn(async move { service.capture("cam-a", "t", 640).await })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        // El único permiso está ocupado: esta captura espera y vence por timeout.
        assert_eq!(service.capture("cam-b", "t", 640).await, Err(SnapshotError::Timeout));
        assert_eq!(holder.await.unwrap(), Err(SnapshotError::Timeout));
    }

    /// Sondea `kill -0` hasta 3 s: el hijo se mata y cosecha en segundo plano.
    async fn process_is_gone(pid: &str) -> bool {
        for _ in 0..30 {
            let alive = std::process::Command::new("kill")
                .args(["-0", pid])
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !alive {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        false
    }
}
