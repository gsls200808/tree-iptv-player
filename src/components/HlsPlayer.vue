<template>
  <div class="player-container">
    <video ref="videoElement" class="video-player" controls playsinline></video>
    <div v-if="error" class="error-message">{{ error }}</div>
    <div v-if="loading" class="loading-message">加载中...</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue';
import Hls from 'hls.js';
import mpegts from 'mpegts.js';
import { getProxyUrl, startRtspProxy, stopRtspProxy } from '../utils/tauriApi';

const props = defineProps<{
  src: string;
}>();

const videoElement = ref<HTMLVideoElement | null>(null);
const error = ref<string>('');
const loading = ref(true);
let hls: Hls | null = null;
let flvPlayer: any = null;
let currentRtspUrl: string | null = null;

// Determine stream type from URL
function getStreamType(url: string): 'flv' | 'hls' | 'rtsp' {
  const lower = url.toLowerCase();
  if (lower.startsWith('rtsp://')) return 'rtsp';
  // Direct .flv URL
  if (lower.includes('.flv')) return 'flv';
  // vodId API paths that are known to serve FLV
  if (lower.includes('vodid=') && (lower.includes('/fhx/') || lower.includes('/fh'))) {
    return 'flv';
  }
  return 'hls';
}

const destroyPlayers = () => {
  if (hls) {
    hls.destroy();
    hls = null;
  }
  if (flvPlayer) {
    try {
      flvPlayer.pause();
      flvPlayer.unload();
      flvPlayer.detachMediaElement();
      flvPlayer.destroy();
    } catch (e) {
      console.warn('Error destroying FLV player:', e);
    }
    flvPlayer = null;
  }
  if (currentRtspUrl) {
    stopRtspProxy(currentRtspUrl).catch((e) => {
      console.warn('Error stopping RTSP proxy:', e);
    });
    currentRtspUrl = null;
  }
};

const initFlvPlayer = async (streamUrl: string) => {
  if (!videoElement.value) return;

  const video = videoElement.value;

  if (!mpegts.isSupported()) {
    error.value = '您的浏览器不支持 FLV 播放';
    loading.value = false;
    return;
  }

  flvPlayer = mpegts.createPlayer(
    {
      type: 'flv',
      isLive: true,
      url: streamUrl,
    },
    {
      enableStashBuffer: false,
      lazyLoad: false,
      autoCleanupSourceBuffer: true,
      autoCleanupMaxBackwardDuration: 15,
      autoCleanupMinBackwardDuration: 10,
    }
  );

  flvPlayer.attachMediaElement(video);
  flvPlayer.load();

  let metadataReceived = false;

  const tryPlay = () => {
    // Muted autoplay is always allowed by browsers
    video.muted = true;
    video
      .play()
      .then(() => {
        console.log('FLV autoplay started (muted)');
        // Unmute after playback begins
        setTimeout(() => {
          video.muted = false;
        }, 100);
      })
      .catch((e) => {
        console.log('Auto-play failed:', e);
      });
  };

  flvPlayer.on(mpegts.Events.METADATA_ARRIVED, () => {
    if (!metadataReceived) {
      console.log('FLV metadata loaded');
      metadataReceived = true;
      loading.value = false;
      tryPlay();
    }
  });

  flvPlayer.on(mpegts.Events.STATISTICS_INFO, () => {
    if (!metadataReceived) {
      metadataReceived = true;
      loading.value = false;
      tryPlay();
    }
  });

  flvPlayer.on(mpegts.Events.ERROR, (errorType: string, errorDetail: string, errorInfo: any) => {
    console.error('FLV error:', errorType, errorDetail, errorInfo);
    if (errorType === 'NetworkError') {
      loading.value = false;
    }
  });

  setTimeout(() => {
    if (!metadataReceived) {
      metadataReceived = true;
      loading.value = false;
      tryPlay();
    }
  }, 5000);
};

const initHlsPlayer = async (streamUrl: string) => {
  if (!videoElement.value) return;

  const video = videoElement.value;

  if (Hls.isSupported()) {
    hls = new Hls({
      lowLatencyMode: false,
      backBufferLength: 90,
    });

    hls.loadSource(streamUrl);
    hls.attachMedia(video);

    hls.on(Hls.Events.MANIFEST_PARSED, () => {
      console.log('HLS manifest loaded');
      loading.value = false;
      video.play().catch((e) => {
        console.log('Auto-play prevented:', e);
      });
    });

    hls.on(Hls.Events.ERROR, (_event, data) => {
      console.error('HLS error:', data);
      if (data.fatal) {
        switch (data.type) {
          case Hls.ErrorTypes.NETWORK_ERROR:
            loading.value = false;
            setTimeout(() => {
              if (hls) hls.startLoad();
            }, 3000);
            break;
          case Hls.ErrorTypes.MEDIA_ERROR:
            error.value = '媒体错误，尝试恢复...';
            hls?.recoverMediaError();
            break;
          default:
            error.value = '播放失败，无法恢复';
            loading.value = false;
            hls?.destroy();
            break;
        }
      }
    });
  } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
    video.src = streamUrl;
    video.addEventListener('loadedmetadata', () => {
      loading.value = false;
      video.play().catch((e) => {
        console.log('Auto-play prevented:', e);
      });
    });
    video.addEventListener('error', () => {
      error.value = '视频加载失败';
      loading.value = false;
    });
  } else {
    error.value = '您的浏览器不支持 HLS 播放';
    loading.value = false;
  }
};

const initPlayer = async () => {
  if (!videoElement.value) return;

  destroyPlayers();
  error.value = '';
  loading.value = true;

  try {
    const originalUrl = props.src;
    const streamType = getStreamType(originalUrl);

    let playUrl = originalUrl;

    if (streamType === 'rtsp') {
      // RTSP: start backend FFmpeg proxy, get local HLS URL
      try {
        console.log('Starting RTSP proxy for:', originalUrl);
        playUrl = await startRtspProxy(originalUrl);
        currentRtspUrl = originalUrl;
        console.log('RTSP proxy URL:', playUrl);
      } catch (e) {
        console.error('RTSP proxy failed:', e);
        error.value = 'RTSP 代理启动失败，请确认已安装 FFmpeg';
        loading.value = false;
        return;
      }
    } else if (originalUrl.startsWith('http://') || originalUrl.startsWith('https://')) {
      // HTTP(S) URLs go through proxy — it follows redirects and detects content type
      try {
        playUrl = await getProxyUrl(originalUrl);
      } catch (e) {
        console.warn('Proxy unavailable, using original URL:', e);
      }
    }

    console.log(`Stream type: ${streamType}, URL: ${playUrl}`);

    if (streamType === 'flv') {
      await initFlvPlayer(playUrl);
    } else {
      // Both HLS and RTSP (transcoded to HLS) play through the HLS player
      await initHlsPlayer(playUrl);
    }
  } catch (e) {
    console.error('Player init error:', e);
    error.value = '播放器初始化失败';
    loading.value = false;
  }
};

onMounted(() => {
  initPlayer();
});

watch(
  () => props.src,
  () => {
    initPlayer();
  }
);

onBeforeUnmount(() => {
  destroyPlayers();
});
</script>

<style scoped>
.player-container {
  position: relative;
  width: 100%;
  background: #000;
  border-radius: 8px;
  overflow: hidden;
}

.video-player {
  width: 100%;
  height: auto;
  display: block;
  max-height: 70vh;
}

.error-message {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  color: #fff;
  background: rgba(255, 0, 0, 0.8);
  padding: 12px 24px;
  border-radius: 4px;
  text-align: center;
}

.loading-message {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  color: #fff;
  background: rgba(0, 0, 0, 0.8);
  padding: 12px 24px;
  border-radius: 4px;
  text-align: center;
}
</style>
