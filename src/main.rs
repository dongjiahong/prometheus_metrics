use std::{env, future::ready, net::SocketAddr, str::FromStr};

use anyhow::Result;
use axum::{
    extract::Extension,
    http::Request,
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
    Router,
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use sea_orm::{entity::*, query::*, DatabaseConnection};
use serde::Deserialize;
use tower::ServiceBuilder;
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use entity::{od_miner_configs, od_trades};

pub mod entity;
pub mod models;

#[derive(Deserialize)]
struct DataCapRespon {
    stats: Vec<NodeStatus>,
    //name: String,
    //deal_count: String,
}

#[derive(Deserialize)]
struct NodeStatus {
    provider: String,
    //total_deal_size: String,
    percent: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "example_todos=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let recorder_handle = setup_metrics_recorder();

    let db = models::connect_mysql(
        "mysql://root:KWFoZKxMZIgOc2NFiQkci7PmzeA@172.16.2.117:3306/offline_deal",
        true,
        tracing::log::LevelFilter::Trace,
    )
    .await;

    let app = Router::new()
        .route("/metrics", get(move || ready(recorder_handle.render())))
        .layer(
            // 这里的layer实用tower的ServiceBuilder而不是layer和route_layer，
            // 这要原因可以参考https://docs.rs/axum/latest/axum/middleware/index.html#ordering
            ServiceBuilder::new()
                .layer(Extension(db))
                .layer(middleware::from_fn(track_deal_jobs_mysql)),
        );

    let port = env::var("PORT").unwrap_or_else(|_| "8976".to_string());
    let addr = SocketAddr::from_str(format!("0.0.0.0:{}", port).as_ref()).unwrap();
    info!("listen on {:?}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn setup_metrics_recorder() -> PrometheusHandle {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("offline_deal_mysql_jobs".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .unwrap()
        .install_recorder()
        .unwrap()
}

async fn track_deal_jobs_mysql<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    debug!("extensions num: {}", req.extensions().len());

    let db: &DatabaseConnection = req.extensions().get().unwrap();
    // 1. 从数据库拉取在交易的miner
    let miners = od_miner_configs::Entity::find()
        .filter(od_miner_configs::Column::Deleted.eq(0))
        .filter(od_miner_configs::Column::MinerApiInfo.is_not_null())
        .all(db)
        .await
        .unwrap();
    // 2. 分别统计不同miner剩下的任务数量
    for miner in miners {
        // 2.1 统计整体任务数量
        let count = od_trades::Entity::find()
            .filter(od_trades::Column::Deleted.eq(0))
            .filter(od_trades::Column::ProposalCid.is_null())
            .filter(od_trades::Column::MinerId.eq(miner.miner_id.clone()))
            .count(db)
            .await
            .unwrap();
        // 2.2 统计[100m,2G)的文件
        let f_100m_to_2g = get_count_specify_data_size(&db, 104857600, 2147483648, &miner.miner_id)
            .await
            .unwrap();
        // 2.3 统计[2G,10G)的文件
        let f_2g_to_10g =
            get_count_specify_data_size(&db, 2147483648, 10737418240, &miner.miner_id)
                .await
                .unwrap();
        // 2.4 统计[10G,16G)的文件
        let f_10g_to_16g =
            get_count_specify_data_size(&db, 10737418240, 17179869184, &miner.miner_id)
                .await
                .unwrap();
        // 2.5 统计[16G,32G)的文件
        let f_16g_to_32g =
            get_count_specify_data_size(&db, 17179869184, 34359738368, &miner.miner_id)
                .await
                .unwrap();

        let lables = [("miner_id", miner.miner_id.to_string())];
        metrics::gauge!("offline_deal_jobs", count as f64, &lables);
        metrics::gauge!("offline_deal_jobs_100m_2G", f_100m_to_2g as f64, &lables);
        metrics::gauge!("offline_deal_jobs_2G_10G", f_2g_to_10g as f64, &lables);
        metrics::gauge!("offline_deal_jobs_10G_16G", f_10g_to_16g as f64, &lables);
        metrics::gauge!("offline_deal_jobs_16G_32G", f_16g_to_32g as f64, &lables);
    }

    // 3. 拉取几点的datacap使用情况
    let url = "https://api.filplus.d.interplanetary.one/api/getDealAllocationStats/f01878897";

    let resp = reqwest::get(url)
        .await
        .unwrap()
        .json::<DataCapRespon>()
        .await
        .unwrap();
    for stat in resp.stats {
        let lables = [("miner_id", stat.provider)];
        metrics::gauge!(
            "offline_deal_datacap_rate",
            stat.percent.parse::<f64>().unwrap(),
            &lables
        );
    }

    next.run(req).await
}

async fn get_count_specify_data_size(
    db: &DatabaseConnection,
    left_size: u64,
    right_size: u64,
    miner_id: &str,
) -> Result<i64> {
    let res: i64 = db
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::MySql,
            format!(
                "select count(*) as count from od_files where deleted = 0
    and data_size >= {}
    and data_size < {}
    and id in (select file_id from od_trades where proposal_cid is null
    and deleted = 0
    and miner_id = '{}')",
                left_size, right_size, miner_id
            ),
        ))
        .await?
        .unwrap()
        .try_get("", "count")?;
    Ok(res)
}
