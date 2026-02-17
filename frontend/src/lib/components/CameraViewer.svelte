<script lang="ts">
	import { onDestroy } from 'svelte';
	import { Maximize2, Minimize2, X, ZoomIn, ZoomOut, RotateCcw, Camera, Circle, Square, Download } from 'lucide-svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import Hls from 'hls.js';

	let {
		open = $bindable(false),
		streamName = ''
	}: {
		open: boolean;
		streamName: string;
	} = $props();

	let videoEl = $state<HTMLVideoElement>(null!);
	let wrapEl = $state<HTMLDivElement>(null!);
	let hlsInstance: Hls | null = null;
	let webrtcPc: RTCPeerConnection | null = null;
	let scale = $state(1);
	let panX = $state(0);
	let panY = $state(0);
	let isDragging = false;
	let dragStartX = 0;
	let dragStartY = 0;
	let panStartX = 0;
	let panStartY = 0;
	let loading = $state(true);
	let errorMsg = $state('');
	let protocol = $state<'WebRTC' | 'HLS' | ''>('');
	let isRecording = $state(false);
	let isFullscreen = $state(false);
	let mediaRecorder: MediaRecorder | null = null;
	let recordedChunks: Blob[] = [];
	let recordingAnimFrame = 0;
	let recordingCanvas: HTMLCanvasElement | null = null;
	let lastTap = 0;
	let touchStartDist = 0;
	let touchStartScale = 1;
	let touchStartPanX = 0;
	let touchStartPanY = 0;
	let touchStartMidX = 0;
	let touchStartMidY = 0;

	const ZOOM_PRESETS = [1, 2, 4, 8];

	$effect(() => {
		if (open && streamName) {
			loading = true;
			errorMsg = '';
			protocol = '';
			scale = 1;
			panX = 0;
			panY = 0;
			const webrtcUrl = `${location.protocol}//${location.hostname}:8889/${streamName}/whep`;
			const hlsUrl = `${location.protocol}//${location.hostname}:8888/${streamName}/index.m3u8`;
			setTimeout(() => {
				if (videoEl) {
					cleanup();
					tryWebRTC(webrtcUrl, hlsUrl);
				}
			}, 50);
		}
		if (!open) {
			stopRecordingInternal();
			cleanup();
		}
	});

	onDestroy(() => {
		stopRecordingInternal();
		cleanup();
	});

	async function tryWebRTC(webrtcUrl: string, hlsUrl: string) {
		try {
			const pc = new RTCPeerConnection({ iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] });
			webrtcPc = pc;
			pc.addTransceiver('video', { direction: 'recvonly' });
			pc.addTransceiver('audio', { direction: 'recvonly' });

			pc.ontrack = (evt) => {
				if (evt.streams?.[0] && videoEl) {
					videoEl.srcObject = evt.streams[0];
					loading = false;
					protocol = 'WebRTC';
					videoEl.play().catch(() => {});
				}
			};

			pc.oniceconnectionstatechange = () => {
				if (pc.iceConnectionState === 'failed' || pc.iceConnectionState === 'disconnected') {
					cleanupWebRTC();
					startHLS(hlsUrl);
				}
			};

			const offer = await pc.createOffer();
			await pc.setLocalDescription(offer);

			await new Promise<void>((resolve) => {
				if (pc.iceGatheringState === 'complete') { resolve(); return; }
				const check = () => { if (pc.iceGatheringState === 'complete') { pc.removeEventListener('icegatheringstatechange', check); resolve(); } };
				pc.addEventListener('icegatheringstatechange', check);
				setTimeout(resolve, 2000);
			});

			const resp = await fetch(webrtcUrl, { method: 'POST', headers: { 'Content-Type': 'application/sdp' }, body: pc.localDescription!.sdp });
			if (!resp.ok) throw new Error(`WHEP ${resp.status}`);
			const answer = await resp.text();
			await pc.setRemoteDescription(new RTCSessionDescription({ type: 'answer', sdp: answer }));

			setTimeout(() => {
				if (videoEl?.srcObject === null && webrtcPc === pc) {
					cleanupWebRTC();
					startHLS(hlsUrl);
				}
			}, 5000);
		} catch {
			cleanupWebRTC();
			startHLS(hlsUrl);
		}
	}

	function startHLS(hlsUrl: string) {
		protocol = 'HLS';
		if (!videoEl) return;
		videoEl.srcObject = null;

		if (Hls.isSupported()) {
			hlsInstance = new Hls({ liveSyncDurationCount: 2, liveMaxLatencyDurationCount: 5, lowLatencyMode: true });
			hlsInstance.loadSource(hlsUrl);
			hlsInstance.attachMedia(videoEl);
			hlsInstance.on(Hls.Events.MANIFEST_PARSED, () => { loading = false; videoEl.play().catch(() => {}); });
			hlsInstance.on(Hls.Events.ERROR, (_e, data) => {
				if (data.fatal) { loading = false; errorMsg = 'No se pudo conectar al stream. Verifica que la cámara esté activa y sincronizada.'; }
			});
		} else if (videoEl.canPlayType('application/vnd.apple.mpegurl')) {
			videoEl.src = hlsUrl;
			videoEl.addEventListener('loadedmetadata', () => { loading = false; videoEl.play().catch(() => {}); }, { once: true });
		} else {
			loading = false;
			errorMsg = 'Tu navegador no soporta HLS ni WebRTC.';
		}
	}

	function cleanupWebRTC() { if (webrtcPc) { try { webrtcPc.close(); } catch {} webrtcPc = null; } }
	function cleanup() {
		if (hlsInstance) { hlsInstance.destroy(); hlsInstance = null; }
		cleanupWebRTC();
		if (videoEl) { videoEl.srcObject = null; videoEl.src = ''; }
	}

	function clampPan() {
		if (!wrapEl) return;
		const w = wrapEl.clientWidth;
		const h = wrapEl.clientHeight;
		const maxX = (scale - 1) * w;
		const maxY = (scale - 1) * h;
		panX = Math.min(0, Math.max(-maxX, panX));
		panY = Math.min(0, Math.max(-maxY, panY));
	}

	function zoomTo(newScale: number, cx?: number, cy?: number) {
		if (!wrapEl) return;
		const rect = wrapEl.getBoundingClientRect();
		const centerX = cx !== undefined ? cx - rect.left : rect.width / 2;
		const centerY = cy !== undefined ? cy - rect.top : rect.height / 2;
		const clamped = Math.min(8, Math.max(1, newScale));
		if (clamped === 1) { scale = 1; panX = 0; panY = 0; return; }
		const ratio = clamped / scale;
		panX = centerX - (centerX - panX) * ratio;
		panY = centerY - (centerY - panY) * ratio;
		scale = clamped;
		clampPan();
	}

	function handleWheel(e: WheelEvent) {
		e.preventDefault();
		const step = e.deltaY < 0 ? 0.3 : -0.3;
		zoomTo(scale + step, e.clientX, e.clientY);
	}

	function handleMouseDown(e: MouseEvent) {
		if (scale <= 1) return;
		isDragging = true;
		dragStartX = e.clientX;
		dragStartY = e.clientY;
		panStartX = panX;
		panStartY = panY;
		wrapEl?.classList.add('dragging');
		e.preventDefault();
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isDragging) return;
		panX = panStartX + (e.clientX - dragStartX);
		panY = panStartY + (e.clientY - dragStartY);
		clampPan();
	}

	function handleMouseUp() {
		if (!isDragging) return;
		isDragging = false;
		wrapEl?.classList.remove('dragging');
	}

	function handleDblClick(e: MouseEvent) {
		if (scale > 1) { scale = 1; panX = 0; panY = 0; }
		else { zoomTo(3, e.clientX, e.clientY); }
	}

	// Touch support
	function getTouchDist(t1: Touch, t2: Touch) {
		return Math.hypot(t2.clientX - t1.clientX, t2.clientY - t1.clientY);
	}

	function handleTouchStart(e: TouchEvent) {
		if (e.touches.length === 2) {
			e.preventDefault();
			touchStartDist = getTouchDist(e.touches[0], e.touches[1]);
			touchStartScale = scale;
			touchStartPanX = panX;
			touchStartPanY = panY;
			touchStartMidX = (e.touches[0].clientX + e.touches[1].clientX) / 2;
			touchStartMidY = (e.touches[0].clientY + e.touches[1].clientY) / 2;
		} else if (e.touches.length === 1 && scale > 1) {
			isDragging = true;
			dragStartX = e.touches[0].clientX;
			dragStartY = e.touches[0].clientY;
			panStartX = panX;
			panStartY = panY;
		}
		// Double-tap detection
		if (e.touches.length === 1) {
			const now = Date.now();
			if (now - lastTap < 300) {
				e.preventDefault();
				if (scale > 1) { scale = 1; panX = 0; panY = 0; }
				else { zoomTo(3, e.touches[0].clientX, e.touches[0].clientY); }
			}
			lastTap = now;
		}
	}

	function handleTouchMove(e: TouchEvent) {
		if (e.touches.length === 2) {
			e.preventDefault();
			const dist = getTouchDist(e.touches[0], e.touches[1]);
			const newScale = Math.min(8, Math.max(1, touchStartScale * (dist / touchStartDist)));
			const midX = (e.touches[0].clientX + e.touches[1].clientX) / 2;
			const midY = (e.touches[0].clientY + e.touches[1].clientY) / 2;
			if (newScale === 1) { scale = 1; panX = 0; panY = 0; }
			else {
				const ratio = newScale / touchStartScale;
				panX = midX - (touchStartMidX - touchStartPanX) * ratio;
				panY = midY - (touchStartMidY - touchStartPanY) * ratio;
				scale = newScale;
				clampPan();
			}
		} else if (e.touches.length === 1 && isDragging) {
			panX = panStartX + (e.touches[0].clientX - dragStartX);
			panY = panStartY + (e.touches[0].clientY - dragStartY);
			clampPan();
		}
	}

	function handleTouchEnd() {
		isDragging = false;
	}

	function toggleFullscreen() {
		if (!wrapEl) return;
		if (!document.fullscreenElement) {
			wrapEl.requestFullscreen().catch(() => {});
			isFullscreen = true;
		} else {
			document.exitFullscreen();
			isFullscreen = false;
		}
	}

	function captureScreenshot() {
		if (!videoEl) return;
		const canvas = document.createElement('canvas');
		canvas.width = videoEl.videoWidth || 1920;
		canvas.height = videoEl.videoHeight || 1080;
		const ctx = canvas.getContext('2d')!;
		ctx.drawImage(videoEl, 0, 0, canvas.width, canvas.height);
		canvas.toBlob((blob) => {
			if (!blob) return;
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `screenshot-${streamName}-${new Date().toISOString().replace(/[:.]/g, '-')}.png`;
			a.click();
			URL.revokeObjectURL(url);
			toast.success('Captura guardada');
		});
	}

	function toggleRecording() {
		if (isRecording) stopRecordingInternal();
		else startRecordingInternal();
	}

	function startRecordingInternal() {
		if (!videoEl) return;
		recordingCanvas = document.createElement('canvas');
		recordingCanvas.width = videoEl.videoWidth || 1920;
		recordingCanvas.height = videoEl.videoHeight || 1080;
		const ctx = recordingCanvas.getContext('2d')!;
		const stream = recordingCanvas.captureStream(20);
		mediaRecorder = new MediaRecorder(stream, { mimeType: 'video/webm;codecs=vp9', videoBitsPerSecond: 2500000 });
		recordedChunks = [];
		mediaRecorder.ondataavailable = (e) => { if (e.data.size > 0) recordedChunks.push(e.data); };
		mediaRecorder.onstop = () => {
			setTimeout(() => {
				const blob = new Blob(recordedChunks, { type: 'video/webm' });
				const url = URL.createObjectURL(blob);
				const a = document.createElement('a');
				a.href = url;
				a.download = `recording-${streamName}-${new Date().toISOString().replace(/[:.]/g, '-')}.webm`;
				a.click();
				URL.revokeObjectURL(url);
				toast.success('Grabación guardada');
			}, 100);
		};

		const drawFrame = () => {
			if (!isRecording || !recordingCanvas) return;
			try { ctx.drawImage(videoEl, 0, 0, recordingCanvas.width, recordingCanvas.height); } catch {}
			recordingAnimFrame = requestAnimationFrame(drawFrame);
		};

		mediaRecorder.start(1000);
		isRecording = true;
		drawFrame();
		toast.info('Grabación iniciada');
	}

	function stopRecordingInternal() {
		if (mediaRecorder && mediaRecorder.state !== 'inactive') {
			mediaRecorder.stop();
			toast.info('Grabación detenida');
		}
		isRecording = false;
		if (recordingAnimFrame) cancelAnimationFrame(recordingAnimFrame);
		recordingCanvas = null;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (!open) return;
		if (e.key === 'Escape') { open = false; return; }
		if (e.key === '+' || e.key === '=') zoomTo(scale + 0.5);
		if (e.key === '-') zoomTo(scale - 0.5);
		if (e.key === '0') { scale = 1; panX = 0; panY = 0; }
		if (e.key === 'f') toggleFullscreen();
	}
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={handleMouseUp} onkeydown={handleKeydown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-50 flex items-center justify-center modal-overlay bg-black/85" onclick={(e) => { if (e.target === e.currentTarget) open = false; }}>
		<div class="bg-surface-alt border border-edge rounded-xl w-full max-w-6xl mx-2 sm:mx-4 fade-in shadow-2xl max-h-[95vh] flex flex-col">
			<!-- Header -->
			<div class="flex items-center justify-between px-4 sm:px-6 py-3 border-b border-edge shrink-0">
				<div class="min-w-0">
					<h3 class="text-base sm:text-lg font-semibold text-content truncate">{streamName}</h3>
					<div class="flex items-center gap-2 text-xs text-content-muted">
						{#if protocol}
							<span class="badge {protocol === 'WebRTC' ? 'badge-success' : 'badge-info'}">{protocol}</span>
						{/if}
						{#if isRecording}
							<span class="badge badge-danger animate-pulse">REC</span>
						{/if}
					</div>
				</div>
				<div class="flex items-center gap-1 shrink-0">
					<button onclick={toggleFullscreen} class="p-2 text-content-muted hover:text-content transition rounded-lg hover:bg-surface-raised" title="Pantalla completa (F)">
						{#if isFullscreen}
							<Minimize2 class="w-4 h-4" />
						{:else}
							<Maximize2 class="w-4 h-4" />
						{/if}
					</button>
					<button onclick={() => (open = false)} class="p-2 text-content-muted hover:text-content transition rounded-lg hover:bg-surface-raised">
						<X class="w-5 h-5" />
					</button>
				</div>
			</div>
			<!-- Video -->
			<div class="p-2 sm:p-4 flex-1 min-h-0 flex flex-col">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div bind:this={wrapEl} class="viewer-container flex-1 relative"
					style="aspect-ratio:16/9;"
					onwheel={handleWheel}
					onmousedown={handleMouseDown}
					ondblclick={handleDblClick}
					ontouchstart={handleTouchStart}
					ontouchmove={handleTouchMove}
					ontouchend={handleTouchEnd}
				>
					{#if loading}
						<div class="absolute inset-0 flex items-center justify-center bg-black/60 z-20">
							<div class="text-center">
								<div class="animate-spin w-10 h-10 border-3 border-white/20 border-t-blue-400 rounded-full mx-auto mb-3"></div>
								<p class="text-sm text-white/70">Conectando al stream...</p>
								<p class="text-xs text-white/40 mt-1">Intentando WebRTC, fallback a HLS</p>
							</div>
						</div>
					{/if}
					<!-- Zoom indicator -->
					{#if scale > 1}
						<span class="absolute top-3 left-3 bg-black/70 text-white px-2.5 py-1 rounded-lg text-xs z-10 backdrop-blur-sm font-medium">
							{scale.toFixed(1)}x
						</span>
					{/if}
					<!-- svelte-ignore a11y_media_has_caption -->
					<video bind:this={videoEl} autoplay muted playsinline
						style="transform: translate({panX}px, {panY}px) scale({scale});">
					</video>
				</div>
				<!-- Controls toolbar -->
				<div class="mt-2 flex items-center justify-between gap-2 flex-wrap">
					<!-- Zoom controls -->
					<div class="flex items-center gap-1">
						<button onclick={() => zoomTo(scale - 0.5)} class="btn btn-ghost p-1.5" title="Alejar (-)">
							<ZoomOut class="w-4 h-4" />
						</button>
						{#each ZOOM_PRESETS as preset}
							<button onclick={() => { zoomTo(preset); }}
								class="px-2 py-1 rounded text-xs font-medium transition
									{Math.abs(scale - preset) < 0.1
									? 'bg-primary text-white'
									: 'text-content-muted hover:text-content hover:bg-surface-raised'}">
								{preset}x
							</button>
						{/each}
						<button onclick={() => zoomTo(scale + 0.5)} class="btn btn-ghost p-1.5" title="Acercar (+)">
							<ZoomIn class="w-4 h-4" />
						</button>
						<button onclick={() => { scale = 1; panX = 0; panY = 0; }} class="btn btn-ghost p-1.5" title="Restablecer (0)">
							<RotateCcw class="w-4 h-4" />
						</button>
					</div>
					<!-- Action controls -->
					<div class="flex items-center gap-1">
						<button onclick={captureScreenshot} class="btn btn-ghost p-1.5" title="Capturar pantalla">
							<Camera class="w-4 h-4" />
						</button>
						<button onclick={toggleRecording}
							class="btn p-1.5 {isRecording ? 'btn-danger' : 'btn-ghost'}"
							title={isRecording ? 'Detener grabación' : 'Grabar video'}>
							{#if isRecording}
								<Square class="w-4 h-4" />
							{:else}
								<Circle class="w-4 h-4" />
							{/if}
						</button>
					</div>
				</div>
				{#if errorMsg}
					<div class="mt-2 text-sm rounded-lg px-4 py-3 text-center badge-danger border" style="border-color: var(--th-badge-danger-bg);">
						{errorMsg}
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
