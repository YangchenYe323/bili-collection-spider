use std::{collections::HashMap, path::Path, str::FromStr as _, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::fs;
use tracing::{error, info};
use tracing_subscriber::{
    filter::Targets, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    lottery: Lottery,
    cookie: Cookie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Lottery {
    act_id: i64,
    lottery_id: i64,
    num_draw: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cookie {
    #[serde(rename = "DedeUserID__ckMd5")]
    dede_userid_ckmd5: String,
    #[serde(rename = "DedeUserID")]
    dede_userid: String,
    #[serde(rename = "SESSDATA")]
    sess_data: String,
    #[serde(rename = "bili_jct")]
    bili_jct: String,
    #[serde(rename = "expires")]
    expires: i64,
}

#[tokio::main]
async fn main() {
    let filter_layer =
        Targets::from_str(std::env::var("RUST_LOG").as_deref().unwrap_or("info")).unwrap();
    let format_layer = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(format_layer)
        .init();

    let config = get_config_from_file("spider.toml").await;

    let client = Client::new();

    info!("获取buvid3");
    let res = client
        .get("https://api.bilibili.com/x/web-frontend/getbuvid")
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let buvid3 = body["data"]["buvid"].as_str().unwrap();
    info!("获取到buvid3={}", buvid3);

    info!("获取收藏集商品ID");
    let res = client
        .get("https://api.bilibili.com/x/vas/dlc_act/act/basic")
        .query(&[
            ("act_id", &config.lottery.act_id.to_string()),
            ("csrf", &config.cookie.bili_jct),
        ])
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let goods_id = body["data"]["lottery_list"][0]["goods_id"]
        .as_i64()
        .unwrap();
    info!("获取到收藏集商品ID={}", goods_id);

    run(&client, &config.lottery, &config.cookie, buvid3, goods_id).await;
}

async fn run(client: &Client, lottery: &Lottery, cookie: &Cookie, buvid3: &str, goods_id: i64) {
    let cookie_str = format!("buvid3={}; DedeUserID={}; DedeUserID__ckMd5={}; bili_ticket_expires={}; SESSDATA={}; bili_jct={};", buvid3, cookie.dede_userid, cookie.dede_userid_ckmd5, cookie.expires, cookie.sess_data, cookie.bili_jct);
    let ten_draw_form = draw_item_form(lottery.act_id, lottery.lottery_id, &cookie.bili_jct, 10);
    let five_draw_form = draw_item_form(lottery.act_id, lottery.lottery_id, &cookie.bili_jct, 5);
    let single_draw_form = draw_item_form(lottery.act_id, lottery.lottery_id, &cookie.bili_jct, 1);

    loop {
        info!("购买收藏集抽取次数 {} 次", lottery.num_draw);
        let form = create_order_form(
            lottery.act_id,
            lottery.lottery_id,
            &cookie.bili_jct,
            goods_id,
            lottery.num_draw,
        );
        let res = client
            .post("https://api.live.bilibili.com/xlive/revenue/v1/order/createOrder")
            .header("Cookie", format!("SESSDATA={}", cookie.sess_data))
            .form(&form)
            .send()
            .await
            .unwrap();
        let body: Value = res.json().await.unwrap();
        if body["code"].as_i64().unwrap() != 0 {
            let s = serde_json::to_string_pretty(&body).unwrap();
            error!("购买抽取次数失败: {}", s);
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }
        info!("成功购买抽取次数 {} 次", lottery.num_draw);
        break;
    }

    let mut total_draw_left = lottery.num_draw;

    info!("开始抽收藏集");
    loop {
        let (num_ten_draws, num_five_draws, num_single_draws) = {
            let tens = total_draw_left / 10;
            let fives = (total_draw_left % 10) / 5;
            let singles = total_draw_left % 5;
            (tens, fives, singles)
        };

        for _ in 0..num_ten_draws {
            info!("抽十次...");
            let data = draw_item(&client, &cookie_str, &ten_draw_form).await;
            total_draw_left -= check_draw_item_response(&data);
            tokio::time::sleep(Duration::from_millis(500)).await
        }

        for _ in 0..num_five_draws {
            info!("抽五次...");
            let data = draw_item(&client, &cookie_str, &five_draw_form).await;
            total_draw_left -= check_draw_item_response(&data);
            tokio::time::sleep(Duration::from_millis(500)).await
        }

        for _ in 0..num_single_draws {
            info!("抽一次...");
            let data = draw_item(&client, &cookie_str, &single_draw_form).await;
            total_draw_left -= check_draw_item_response(&data);
            tokio::time::sleep(Duration::from_millis(500)).await
        }

        if total_draw_left == 0 {
            info!("全部抽取完毕");
            return;
        }
    }
}

// 使用B币购买 *goods_num* 个抽取次数
fn create_order_form(
    act_id: i64,
    lottery_id: i64,
    bili_jct: &str,
    goods_id: i64,
    goods_num: i32,
) -> HashMap<String, String> {
    let mut form_data = HashMap::new();

    // Add all fields to the HashMap
    form_data.insert("area_id".to_string(), "".to_string());
    form_data.insert(
        "biz_extra".to_string(),
        json!({
            "activity_id": act_id,
            "lottery_id":lottery_id,
        })
        .to_string(),
    );
    form_data.insert("biz_source".to_string(), "1".to_string());
    form_data.insert("build".to_string(), "0".to_string());
    form_data.insert("common_bp".to_string(), "0".to_string());
    form_data.insert("context_id".to_string(), "0".to_string());
    form_data.insert("context_type".to_string(), "103".to_string());
    form_data.insert("csrf".to_string(), bili_jct.to_string());
    form_data.insert("goods_id".to_string(), goods_id.to_string());
    form_data.insert("goods_num".to_string(), goods_num.to_string());
    form_data.insert("ios_bp".to_string(), "0".to_string());

    // 需要的bp数量
    let num_bp = goods_num * 9900;
    form_data.insert("pay_bp".to_string(), num_bp.to_string());
    form_data.insert("platform".to_string(), "pc".to_string());

    form_data
}

// 抽 *lottery_num* 次卡
fn draw_item_form(
    act_id: i64,
    lottery_id: i64,
    bili_jct: &str,
    lottery_num: i32,
) -> HashMap<String, String> {
    let mut form_data = HashMap::new();

    form_data.insert("act_id".to_string(), act_id.to_string());
    form_data.insert("csrf".to_string(), bili_jct.to_string());
    form_data.insert("lottery_id".to_string(), lottery_id.to_string());
    form_data.insert("lottery_num".to_string(), lottery_num.to_string());

    form_data
}

async fn get_config_from_file(path: impl AsRef<Path>) -> Config {
    let content = fs::read_to_string(path).await.unwrap();
    toml::from_str(&content).unwrap()
}

async fn draw_item<T: Serialize + ?Sized>(client: &Client, cookie: &str, form: &T) -> Value {
    let res = client
        .post("https://api.bilibili.com/x/vas/dlc_act/lottery/draw_item")
        .header("Origin", "https://www.bilibili.com")
        .header("Referer", "https://www.bilibili.com")
        .header("Cookie", cookie)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .form(form)
        .send()
        .await
        .unwrap();

    let body = res.json().await.unwrap();

    body
}

fn check_draw_item_response(body: &Value) -> i32 {
    let Some(code) = body.get("code").and_then(Value::as_i64) else {
        error!("请求错误: {:?}", body);
        return 0;
    };

    if code != 0 {
        error!("请求错误: {:?}", body);
        return 0;
    }

    let error_code = body["data"]["err_code"].as_i64().unwrap();
    if error_code != 0 {
        error!("抽奖失败: {:?}", body);
        return 0;
    }

    if let Some(items) = body["data"]["item_list"].as_array() {
        for item in items {
            let card_name = item["card_item"]["card_type_info"]["name"]
                .as_str()
                .unwrap();
            let card_chance = item["card_item"]["card_chance"].as_f64().unwrap();
            info!("抽到了 {}, 概率 {} !", card_name, card_chance);
        }

        return items.len() as i32;
    }

    return 0;
}
