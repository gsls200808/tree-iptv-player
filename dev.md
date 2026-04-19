# 开发指南

## RTSP测试流搭建

https://github.com/bluenviron/mediamtx/releases
下载1.12.3版本 最新的1.17.1不行
下载安装
UDP推流

```
ffmpeg -re -i input.mp4 -c copy -f rtsp rtsp://127.0.0.1:8554/stream
```

TCP推流

```
ffmpeg -re -i input.mp4 -c copy -rtsp_transport tcp -f rtsp rtsp://127.0.0.1:8554/stream
```

循环推流

```
ffmpeg -re -stream_loop -1 -i input.mp4 -c copy -f rtsp rtsp://127.0.0.1:8554/stream
```

播放

```
rtsp://127.0.0.1:8554/stream
rtsp://192.168.88.1:8554/stream
```





## github action 自动构建



提交内容

```
git add .
git commit -m "something"
git push 
```

触发构建

```
Cargo.toml 文件的version = "0.1.7" 改成对应版本
# 触发
git tag -d v0.1.7
git push origin --delete v0.1.7
git tag v0.1.7
git push origin v0.1.7

```

