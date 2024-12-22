# bili-collection-spider - 自动抽B站收藏集工具

## Usage
在项目根目录下创建 `spider.toml`, 内容示例如下:

```toml
[lottery]
act_id = 100309
lottery_id = 100310
num_draw = 1

[cookie]
SESSDATA = "SESSDATA"
bili_jct = "BILI_JCT"
DedeUserID = "DDDDD"
DedeUserID__ckMd5 = "DDDDD"
expires = 1748534742
```

然后运行主程序:

```
cargo run --release
```

这个程序会购买 `num_draw` 次抽取并且全部抽掉 : )

```
2024-12-22T01:31:37.202469Z  INFO bili_collection_spider: 获取buvid3
2024-12-22T01:31:37.721862Z  INFO bili_collection_spider: 获取到buvid3=5AE2BBD1-FF45-17D0-2B9D-CC47D2A7A8ED96590infoc
2024-12-22T01:31:37.722018Z  INFO bili_collection_spider: 获取收藏集商品ID
2024-12-22T01:31:38.034894Z  INFO bili_collection_spider: 获取到收藏集商品ID=22298314699264
2024-12-22T01:31:38.035105Z  INFO bili_collection_spider: 购买收藏集抽取次数 1 次
2024-12-22T01:31:38.696249Z  INFO bili_collection_spider: 成功购买抽取次数 1 次
2024-12-22T01:31:38.696398Z  INFO bili_collection_spider: 开始抽收藏集
2024-12-22T01:31:38.696559Z  INFO bili_collection_spider: 抽一次...
2024-12-22T01:31:39.072275Z  INFO bili_collection_spider: 抽到了 兰兰爱你~ !
```
