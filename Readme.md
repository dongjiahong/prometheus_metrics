# 项目介绍
该项目用来发送自定义的metrics，项目通过查询mysql数据库从而获得不同的miner的剩余交易的数量

## 启动
使用systemctl
```systemctl
[Unit]
Description=deal jobs mysql

[Service]
User=root
Type=simple
ExecStart=/usr/local/bin/deal_jobs
Restart=on-failure
RestartSec=10s

[Install]
WantedBy=multi-user.target
```

## 数据库依赖
使用sea-orm
1. 安装sea-orm命令行工具
    ```bash
    cargo install sea-orm-cli
    ```
2. 生成对应的表结构
    ```bash
    sea-orm-cli generate entity -u mysql://root:root123@127.0.0.0.1:3306/db src/entity
    ```
## prometheus配置
```yml
 - job_name: "offline_deal_jobs"
      scrape_interval: 10m # 30s拉取一次信息
      static_configs:
        - targets: ["127.0.0.1:8976"]
```
