github action 自动构建



提交内容

```
git add .
git commit -m "something"
git push 
```

触发构建

```
# 触发
git tag -d v0.1.0
git push origin --delete v0.1.0
git tag v0.1.0
git push origin v0.1.0

```

