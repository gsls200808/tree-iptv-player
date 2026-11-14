# 小树IPTV播放器

小树IPTV播放器是一款支持在线m3u播放列表的IPTV播放器，采用Tauri + Vue + TypeScripts技术架构开发

## 背景

市面上的iptv播放器自己用起来不那么顺手，https://github.com/4gray/iptvnator 这个项目做的很好了，但是还不符合我的使用习惯，打算自己写一个

## 功能

### 播放

- 播放 m3u 播放列表
- 播放 m3u8 单个链接
- HTTP-FLV 支持
- RTSP 支持（依赖 ffmpeg）
- MPEG-TS 直播流支持：rtp2httpd、udpxy 等组播转 HTTP 的地址（`/rtp/`、`/udp/`、`/udpxy/`、`.ts`）
- MPEG-DASH（`.mpd`）支持
- 隔行扫描自动反交错：全局开关，默认开启，仅对检测为隔行扫描的流生效，逐行流不受影响
- 特殊音轨兼容：E-AC3、AC-3、多声道等浏览器不支持的音轨，自动转码为 AAC 后播放

### 播放列表

- m3u / m3u8 播放列表
- DIYP TXT（含 `,#genre#` 分组标记）
- 标准 TXT（`频道名,地址` 逐行格式）
- 频道分组显示

### EPG

- XMLTV 格式
- DIYP 格式（JSON）
- 节目表显示、当前节目高亮
- PLTV 回放：对 URL 含 `PLTV` / `TVOD` 的频道，可通过 `?playseek=` 回看已结束的节目

### 待排期

- m3u播放列表 kodi回看格式 tivimate回看格式兼容
- MCE遥控器控制兼容
- xbox手柄控制兼容
- 原生 rtp:// 组播地址（浏览器无法直连组播，需经由 rtp2httpd / udpxy 转 HTTP）
- m3u DRM 支持
- mpd DRM 支持



# 开发和运行

## 开发

开发环境运行

```
# 安装依赖
npm install

# 启动开发
npm run tauri dev
```

## 打包二进制

打包成二进制运行

```
npm run tauri build
```

## 运行依赖

以下功能依赖 ffmpeg（含 ffprobe）：
- RTSP 播放
- 隔行扫描检测与反交错转码
- 特殊音轨（E-AC3 / AC-3 / 多声道）转 AAC

未安装时上述功能不可用，其他格式不受影响。安装方式：

### Windows下载安装ffmpeg

下载链接[Releases · BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds/releases) 

搜索-win64-gpl.zip 找到 ffmpeg-N-124032-gfcffc0e1c5-win64-gpl.zip

解压 将bin目录加到path环境变量

验证

```
ffmpeg -version
```

### linux下载安装ffmpeg

Ubuntu / Debian 系列：

```
sudo apt update
sudo apt install ffmpeg
```

CentOS / RHEL 系列：

```
sudo dnf install epel-release
sudo dnf install ffmpeg ffmpeg-devel
```

### macos下载安装ffmpeg

homebrew安装

```
# 更新
$ brew update
# 升级到最新的 Homebrew
$ brew upgrade
# 在 Homebrew 上安装最新版本的 FFmpeg
$ brew install ffmpeg
```

