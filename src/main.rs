use std::{future::ready, net::SocketAddr, str::FromStr};

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
use tower::ServiceBuilder;
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use entity::{od_miner_configs, od_trades};

pub mod entity;
pub mod models;

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

    let addr = SocketAddr::from_str("0.0.0.0:8976").unwrap();
    tracing::info!("listen on {:?}", addr);
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
        let count = od_trades::Entity::find()
            .filter(od_trades::Column::Deleted.eq(0))
            .filter(od_trades::Column::ProposalCid.is_null())
            .filter(od_trades::Column::MinerId.eq(miner.miner_id.clone()))
            .count(db)
            .await
            .unwrap();
        let lables = [("miner_id", miner.miner_id.to_string())];
        metrics::gauge!("offline_deal_jobs", count as f64, &lables);
    }

    next.run(req).await
}
