# 小树IPTV播放器

小树IPTV播放器是一款支持在线m3u播放列表的IPTV播放器，采用Tauri + Vue + TypeScripts技术架构开发

## 背景

市面上的iptv播放器自己用起来不那么顺手，https://github.com/4gray/iptvnator 这个项目做的很好了，但是还不符合我的使用习惯，打算自己写一个

## 功能

- 播放m3u播放列表
- 播放m3u8单个链接
- EPG显示 支持xmltv diyp 格式 
- PLTV格式回放
- HTTP-FLV 支持
- RTSP 支持
- m3u播放列表 kodi回看格式 tivimate回看格式兼容 待排期
- MCE遥控器控制兼容 待排期
- xbox手柄控制兼容 待排期
- rtp支持 待排期
- mpd格式支持 待排期
- 播放列表diyp txt  普通txt 等格式兼容 待排期
- m3u DRM 支持 待排期
- mpd DRM 支持 待排期



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

RTSP播放依赖ffmpeg，可以安装下面的方式安装

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

