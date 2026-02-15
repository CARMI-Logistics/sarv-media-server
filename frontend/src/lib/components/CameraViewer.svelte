<script lang="ts">
	import { onDestroy } from 'svelte';
	import { Maximize2, X } from 'lucide-svelte';
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
	let urlInfo = $state('');
	let isRecording = $state(false);
	let mediaRecorder: MediaRecorder | null = null;
	let recordedChunks: Blob[] = [];
	let recordingAnimFrame = 0;
	let recordingCanvas: HTMLCanvasElement | null = null;

	$effect(() => {
		if (open && streamName) {
			loading = true;
			errorMsg = '';
			scale = 1;
			panX = 0;
			panY = 0;
			const webrtcUrl = `${location.protocol}//${location.hostname}:8889/${streamName}/whep`;
			const hlsUrl = `${location.protocol}//${location.hostname}:8888/${streamName}/index.m3u8`;
			urlInfo = `WebRTC: ${webrtcUrl}`;
			// Wait for DOM to render video element
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
			urlInfo = `WebRTC: ${webrtcUrl}`;

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
		urlInfo = `HLS: ${hlsUrl}`;
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

	function zoom(delta: number, mx?: number, my?: number) {
		if (!videoEl) return;
		const rect = videoEl.getBoundingClientRect();
		const cx = mx !== undefined ? mx - rect.left : rect.width / 2;
		const cy = my !== undefined ? my - rect.top : rect.height / 2;
		const oldScale = scale;
		const newScale = Math.min(8, Math.max(1, scale + delta));
		if (newScale === 1) { scale = 1; panX = 0; panY = 0; }
		else {
			const ratio = newScale / oldScale;
			panX = cx - (cx - panX) * ratio;
			panY = cy - (cy - panY) * ratio;
			scale = newScale;
			clampPan();
		}
	}

	function handleWheel(e: WheelEvent) {
		e.preventDefault();
		zoom(e.deltaY < 0 ? 0.2 : -0.2, e.clientX, e.clientY);
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

	function toggleFullscreen() {
		if (!wrapEl) return;
		if (!document.fullscreenElement) wrapEl.requestFullscreen().catch(() => {});
		else document.exitFullscreen();
	}

	function captureScreenshot() {
		if (!videoEl) return;
		const canvas = document.createElement('canvas');
		canvas.width = videoEl.videoWidth;
		canvas.height = videoEl.videoHeight;
		const ctx = canvas.getContext('2d')!;
		ctx.save();
		ctx.translate(canvas.width / 2, canvas.height / 2);
		ctx.scale(scale, scale);
		ctx.translate(-canvas.width / 2 + panX, -canvas.height / 2 + panY);
		ctx.drawImage(videoEl, 0, 0, canvas.width, canvas.height);
		ctx.restore();
		canvas.toBlob((blob) => {
			if (!blob) return;
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `screenshot-${new Date().toISOString().replace(/[:.]/g, '-')}.png`;
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
		recordingCanvas.width = videoEl.videoWidth;
		recordingCanvas.height = videoEl.videoHeight;
		const ctx = recordingCanvas.getContext('2d')!;
		const stream = recordingCanvas.captureStream(20);
		mediaRecorder = new MediaRecorder(stream, { mimeType: 'video/webm;codecs=vp9', videoBitsPerSecond: 1500000 });
		recordedChunks = [];
		mediaRecorder.ondataavailable = (e) => { if (e.data.size > 0) recordedChunks.push(e.data); };
		mediaRecorder.onstop = () => {
			setTimeout(() => {
				const blob = new Blob(recordedChunks, { type: 'video/webm' });
				const url = URL.createObjectURL(blob);
				const a = document.createElement('a');
				a.href = url;
				a.download = `recording-${new Date().toISOString().replace(/[:.]/g, '-')}.webm`;
				a.click();
				URL.revokeObjectURL(url);
				toast.success('Grabación guardada');
			}, 100);
		};

		const drawFrame = () => {
			if (!isRecording || !recordingCanvas) return;
			try {
				ctx.save();
				ctx.translate(recordingCanvas.width / 2, recordingCanvas.height / 2);
				ctx.scale(scale, scale);
				ctx.translate(-recordingCanvas.width / 2 + panX, -recordingCanvas.height / 2 + panY);
				ctx.drawImage(videoEl, 0, 0, recordingCanvas.width, recordingCanvas.height);
				ctx.restore();
			} catch {}
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
		if (e.key === 'Escape' && open) { open = false; }
	}
</script>

<svelte:window onmousemove={handleMouseMove} onmouseup={handleMouseUp} onkeydown={handleKeydown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-50 flex items-center justify-center modal-overlay bg-black/80" onclick={(e) => { if (e.target === e.currentTarget) open = false; }}>
		<div class="bg-gray-900 border border-gray-700 rounded-xl w-full max-w-5xl mx-4 fade-in shadow-2xl">
			<div class="flex items-center justify-between px-6 py-3 border-b border-gray-800">
				<div>
					<h3 class="text-lg font-semibold text-white">{streamName}</h3>
					<p class="text-xs text-gray-500 truncate max-w-md">{urlInfo}</p>
				</div>
				<div class="flex items-center gap-2">
					<button onclick={toggleFullscreen} class="p-2 text-gray-400 hover:text-white transition rounded-lg hover:bg-gray-800" title="Pantalla completa">
						<Maximize2 class="w-4 h-4" />
					</button>
					<button onclick={() => (open = false)} class="p-2 text-gray-400 hover:text-white transition rounded-lg hover:bg-gray-800">
						<X class="w-5 h-5" />
					</button>
				</div>
			</div>
			<div class="p-4">
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div bind:this={wrapEl} class="viewer-container" style="aspect-ratio:16/9;" onwheel={handleWheel} onmousedown={handleMouseDown}>
					{#if loading}
						<div class="absolute inset-0 flex items-center justify-center bg-black/50 z-5">
							<div class="text-center text-gray-400">
								<div class="animate-spin w-8 h-8 border-2 border-gray-600 border-t-blue-500 rounded-full mx-auto mb-2"></div>
								<p class="text-sm">Conectando al stream...</p>
							</div>
						</div>
					{/if}
					<span class="absolute top-3 left-3 bg-black/70 text-white px-2.5 py-0.5 rounded-md text-xs z-10 backdrop-blur-sm">
						{scale.toFixed(1)}x
					</span>
					<!-- svelte-ignore a11y_media_has_caption -->
					<video bind:this={videoEl} autoplay muted playsinline style="transform: translate({panX}px, {panY}px) scale({scale})"></video>
					<div class="absolute bottom-3 right-3 flex gap-1 z-10 flex-wrap max-w-[280px]">
						<button onclick={() => zoom(-0.25)} title="Alejar"
							class="w-9 h-9 flex items-center justify-center bg-black/70 border border-white/20 rounded-lg text-white cursor-pointer text-sm backdrop-blur-sm hover:bg-blue-600/60 transition">−</button>
						<button onclick={() => { scale = 1; panX = 0; panY = 0; }} title="Restablecer"
							class="w-9 h-9 flex items-center justify-center bg-black/70 border border-white/20 rounded-lg text-white cursor-pointer text-sm backdrop-blur-sm hover:bg-blue-600/60 transition">⊙</button>
						<button onclick={() => zoom(0.25)} title="Acercar"
							class="w-9 h-9 flex items-center justify-center bg-black/70 border border-white/20 rounded-lg text-white cursor-pointer text-sm backdrop-blur-sm hover:bg-blue-600/60 transition">+</button>
						<button onclick={captureScreenshot} title="Capturar pantalla"
							class="w-9 h-9 flex items-center justify-center bg-black/70 border border-white/20 rounded-lg text-white cursor-pointer text-sm backdrop-blur-sm hover:bg-blue-600/60 transition">📷</button>
						<button onclick={toggleRecording} title="Grabar video"
							class="w-9 h-9 flex items-center justify-center border border-white/20 rounded-lg text-white cursor-pointer text-sm backdrop-blur-sm transition {isRecording ? 'bg-red-600/80 animate-pulse' : 'bg-black/70 hover:bg-blue-600/60'}">
							{isRecording ? '⏹️' : '🔴'}
						</button>
					</div>
				</div>
				{#if errorMsg}
					<div class="mt-3 text-red-400 text-sm bg-red-900/20 border border-red-800 rounded-lg px-4 py-3 text-center">{errorMsg}</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
