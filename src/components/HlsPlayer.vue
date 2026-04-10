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
import { getProxyUrl } from '../utils/tauriApi';

const props = defineProps<{
  src: string;
}>();

const videoElement = ref<HTMLVideoElement | null>(null);
const error = ref<string>('');
const loading = ref(true);
let hls: Hls | null = null;

const initPlayer = async () => {
  if (!videoElement.value) return;

  if (hls) {
    hls.destroy();
    hls = null;
  }

  error.value = '';
  loading.value = true;

  const video = videoElement.value;

  try {
    let streamUrl = props.src;

    if (props.src.startsWith('http://') || props.src.startsWith('https://')) {
      try {
        streamUrl = await getProxyUrl(props.src);
        console.log('Using proxy URL:', streamUrl);
      } catch (e) {
        console.warn('Failed to get proxy URL, using original:', e);
      }
    }

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
        video.play().catch(e => {
          console.log('Auto-play prevented:', e);
        });
      });

      hls.on(Hls.Events.ERROR, (_event, data) => {
        console.error('HLS error:', data);
        if (data.fatal) {
          switch (data.type) {
            case Hls.ErrorTypes.NETWORK_ERROR:
              // error.value = '网络错误，请检查流地址是否正确';
              // 不显示错误提示，静默重试
              loading.value = false;
              setTimeout(() => {
                if (hls) {
                  hls.startLoad();
                }
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
        video.play().catch(e => {
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
  } catch (e) {
    console.error('Player initialization error:', e);
    error.value = '播放器初始化失败';
    loading.value = false;
  }
};

onMounted(() => {
  initPlayer();
});

watch(() => props.src, () => {
  initPlayer();
});

onBeforeUnmount(() => {
  if (hls) {
    hls.destroy();
    hls = null;
  }
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

