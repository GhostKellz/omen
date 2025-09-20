# 🚀 GhostLLM Deployment Guide

Complete deployment documentation for all environments and platforms.

## 📋 Overview

GhostLLM supports multiple deployment strategies:

- **🐳 Docker/Podman** - Recommended for most deployments
- **⚡ Bolt** - LXC/Proxmox container deployments
- **☸️ Kubernetes** - Production cluster deployments
- **🔧 Manual** - Direct binary installation
- **☁️ Cloud** - AWS, GCP, Azure ready

## 🐳 Docker Deployment (Recommended)

### Quick Start

```bash
# Clone repository
git clone https://github.com/yourusername/ghostllm
cd ghostllm

# Configure environment
cp .env.example .env
# Edit .env with your settings

# Start all services
docker-compose up -d

# Verify deployment
curl http://localhost:8080/health
```

### Production Docker Setup

**1. Environment Configuration**

Create production `.env`:

```bash
# Core Configuration
NODE_ENV=production
GHOSTLLM_ENV=production
LOG_LEVEL=info

# Database
DATABASE_URL=postgresql://ghostllm:secure_password@postgres:5432/ghostllm
REDIS_URL=redis://redis:6379

# Security
JWT_SECRET=your-very-long-random-secret-key-here
ADMIN_API_KEY=admin-key-for-management-access
ENCRYPTION_KEY=32-byte-encryption-key-for-sensitive-data

# API Providers
OPENAI_API_KEY=sk-your-openai-key
ANTHROPIC_API_KEY=sk-ant-your-anthropic-key
GOOGLE_API_KEY=your-google-api-key

# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
PROXY_TIMEOUT=30

# Features
ENABLE_AUTH=true
ENABLE_RATE_LIMITING=true
ENABLE_CACHING=true
ENABLE_ANALYTICS=true
ENABLE_BILLING=true

# Monitoring
PROMETHEUS_ENABLED=true
METRICS_PORT=9090

# SSL/TLS
SSL_ENABLED=true
SSL_CERT_PATH=/etc/ssl/certs/ghostllm.pem
SSL_KEY_PATH=/etc/ssl/private/ghostllm.key
```

**2. Production Docker Compose**

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  ghostllm:
    image: ghostllm:latest
    restart: unless-stopped
    environment:
      - NODE_ENV=production
    env_file:
      - .env
    ports:
      - "8080:8080"
      - "9090:9090"  # Metrics
    depends_on:
      - postgres
      - redis
    volumes:
      - ./ssl:/etc/ssl
      - ./logs:/app/logs
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '1.0'
        reservations:
          memory: 1G
          cpus: '0.5'

  postgres:
    image: postgres:15-alpine
    restart: unless-stopped
    environment:
      POSTGRES_DB: ghostllm
      POSTGRES_USER: ghostllm
      POSTGRES_PASSWORD: ${DATABASE_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./database/init.sql:/docker-entrypoint-initdb.d/init.sql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ghostllm"]
      interval: 30s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    command: redis-server --appendonly yes --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 30s
      timeout: 5s
      retries: 5

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf
      - ./nginx/ssl:/etc/nginx/ssl
      - ./nginx/logs:/var/log/nginx
    depends_on:
      - ghostllm

  # Optional: Monitoring Stack
  prometheus:
    image: prom/prometheus:latest
    restart: unless-stopped
    ports:
      - "9091:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus
    profiles:
      - monitoring

  grafana:
    image: grafana/grafana:latest
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD}
    volumes:
      - grafana_data:/var/lib/grafana
      - ./monitoring/grafana:/etc/grafana/provisioning
    profiles:
      - monitoring

volumes:
  postgres_data:
  redis_data:
  prometheus_data:
  grafana_data:

networks:
  default:
    name: ghostllm-network
```

**3. Deploy Production Stack**

```bash
# Build production image
docker build -t ghostllm:latest .

# Start production services
docker-compose -f docker-compose.prod.yml up -d

# Start with monitoring
docker-compose -f docker-compose.prod.yml --profile monitoring up -d

# Verify deployment
docker-compose -f docker-compose.prod.yml ps
curl https://your-domain.com/health
```

## ⚡ Bolt Deployment (LXC/Proxmox)

Perfect for self-hosted deployments on Proxmox or LXC containers.

### Bolt Configuration

**Boltfile:**
```yaml
# Boltfile
name: ghostllm
version: 0.3.0

services:
  ghostllm:
    image: "rust:1.82-bookworm"
    ports:
      - "8080:8080"
      - "4433:4433"
    environment:
      - "RUST_LOG=info"
    volumes:
      - "./:/app"
      - "cargo_cache:/usr/local/cargo"
    working_dir: "/app"
    command: "cargo run --release --bin ghostllm-proxy -- serve"
    depends_on:
      - postgres
      - redis

  postgres:
    image: "postgres:15-alpine"
    environment:
      - "POSTGRES_DB=ghostllm"
      - "POSTGRES_USER=ghostllm"
      - "POSTGRES_PASSWORD=ghostllm123"
    volumes:
      - "postgres_data:/var/lib/postgresql/data"
      - "./database/init.sql:/docker-entrypoint-initdb.d/init.sql"
    ports:
      - "5432:5432"

  redis:
    image: "redis:7-alpine"
    volumes:
      - "redis_data:/data"
    ports:
      - "6379:6379"
    command: "redis-server --appendonly yes"

  nginx:
    image: "nginx:alpine"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - "./nginx/nginx.conf:/etc/nginx/nginx.conf"
      - "./nginx/ssl:/etc/nginx/ssl"
    depends_on:
      - ghostllm

profiles:
  development:
    services:
      ghostllm:
        command: "cargo run --bin ghostllm-proxy -- serve --dev"
        environment:
          - "RUST_LOG=debug"

  production:
    services:
      ghostllm:
        restart: unless-stopped
        deploy:
          replicas: 2
          resources:
            limits:
              memory: 2G
              cpus: 1.0

  monitoring:
    services:
      prometheus:
        image: "prom/prometheus:latest"
        ports:
          - "9090:9090"
        volumes:
          - "./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml"

      grafana:
        image: "grafana/grafana:latest"
        ports:
          - "3000:3000"
        environment:
          - "GF_SECURITY_ADMIN_PASSWORD=admin123"

volumes:
  postgres_data:
  redis_data:
  cargo_cache:
```

### Bolt Deployment Commands

```bash
# Development deployment
bolt up

# Production deployment
bolt up production

# With monitoring
bolt up production monitoring

# Scale services
bolt scale ghostllm=3 redis=2

# View logs
bolt logs ghostllm

# Stop services
bolt down

# Update and redeploy
bolt pull && bolt up
```

### Proxmox LXC Setup

**1. Create LXC Container**

```bash
# On Proxmox host
pct create 100 local:vztmpl/ubuntu-22.04-standard_22.04-1_amd64.tar.zst \
  --hostname ghostllm \
  --memory 4096 \
  --swap 2048 \
  --cores 2 \
  --rootfs local-lvm:20 \
  --net0 name=eth0,bridge=vmbr0,ip=192.168.1.100/24,gw=192.168.1.1 \
  --features nesting=1,keyctl=1

# Start container
pct start 100

# Enter container
pct enter 100
```

**2. Install Dependencies**

```bash
# Update system
apt update && apt upgrade -y

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Install Bolt
curl -sSL https://bolt.run/install.sh | bash

# Install Rust (for development)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**3. Deploy GhostLLM**

```bash
# Clone repository
git clone https://github.com/yourusername/ghostllm
cd ghostllm

# Configure environment
cp .env.example .env
# Edit .env file

# Deploy with Bolt
bolt up production

# Configure firewall
ufw allow 80
ufw allow 443
ufw allow 8080
ufw enable
```

## ☸️ Kubernetes Deployment

Enterprise-grade Kubernetes deployment with Helm charts.

### Prerequisites

```bash
# Install Helm
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

# Add GhostLLM Helm repository
helm repo add ghostllm https://charts.ghostllm.io
helm repo update
```

### Basic Deployment

```bash
# Create namespace
kubectl create namespace ghostllm

# Install with default values
helm install ghostllm ghostllm/ghostllm \
  --namespace ghostllm \
  --set config.adminApiKey=your-admin-key
```

### Production Deployment

**values.yaml:**
```yaml
# Helm values for production
replicaCount: 3

image:
  repository: ghostllm/ghostllm
  tag: "0.3.0"
  pullPolicy: IfNotPresent

service:
  type: LoadBalancer
  port: 80
  targetPort: 8080

ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
  hosts:
    - host: api.ghostllm.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: ghostllm-tls
      hosts:
        - api.ghostllm.com

config:
  adminApiKey: "your-secure-admin-key"
  jwtSecret: "your-jwt-secret"
  providers:
    openai:
      apiKey: "sk-your-openai-key"
    anthropic:
      apiKey: "sk-ant-your-anthropic-key"

postgresql:
  enabled: true
  auth:
    postgresPassword: "postgres-password"
    database: "ghostllm"
  persistence:
    enabled: true
    size: 50Gi

redis:
  enabled: true
  auth:
    enabled: true
    password: "redis-password"
  persistence:
    enabled: true
    size: 10Gi

resources:
  limits:
    cpu: 2000m
    memory: 4Gi
  requests:
    cpu: 500m
    memory: 1Gi

autoscaling:
  enabled: true
  minReplicas: 2
  maxReplicas: 10
  targetCPUUtilizationPercentage: 70
  targetMemoryUtilizationPercentage: 80

monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
  grafana:
    enabled: true
    adminPassword: "grafana-password"
```

**Deploy:**
```bash
# Install with custom values
helm install ghostllm ghostllm/ghostllm \
  --namespace ghostllm \
  --values values.yaml

# Upgrade deployment
helm upgrade ghostllm ghostllm/ghostllm \
  --namespace ghostllm \
  --values values.yaml

# Check status
kubectl get pods -n ghostllm
kubectl get svc -n ghostllm
```

### Custom Kubernetes Manifests

**deployment.yaml:**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ghostllm
  namespace: ghostllm
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ghostllm
  template:
    metadata:
      labels:
        app: ghostllm
    spec:
      containers:
      - name: ghostllm
        image: ghostllm:0.3.0
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ghostllm-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: ghostllm-secrets
              key: redis-url
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
```

## 🔧 Manual Installation

For direct binary installation on servers.

### System Requirements

- **OS:** Linux (Ubuntu 20.04+, RHEL 8+, CentOS 8+)
- **RAM:** 2GB minimum, 4GB recommended
- **CPU:** 1 core minimum, 2+ recommended
- **Disk:** 10GB available space
- **Network:** Internet access for provider APIs

### Binary Installation

```bash
# Download latest release
curl -L https://github.com/yourusername/ghostllm/releases/latest/download/ghostllm-linux-x86_64.tar.gz | tar xz

# Install binary
sudo mv ghostllm-proxy /usr/local/bin/
sudo chmod +x /usr/local/bin/ghostllm-proxy

# Create user
sudo useradd --system --home /var/lib/ghostllm --shell /bin/false ghostllm

# Create directories
sudo mkdir -p /etc/ghostllm /var/lib/ghostllm /var/log/ghostllm
sudo chown ghostllm:ghostllm /var/lib/ghostllm /var/log/ghostllm
```

### Configuration

**Create `/etc/ghostllm/config.toml`:**
```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://ghostllm:password@localhost:5432/ghostllm"

[redis]
url = "redis://localhost:6379"

[auth]
jwt_secret = "your-jwt-secret"
admin_api_key = "your-admin-key"

[providers.openai]
enabled = true
api_key = "sk-your-openai-key"

[providers.anthropic]
enabled = true
api_key = "sk-ant-your-anthropic-key"

[logging]
level = "info"
file = "/var/log/ghostllm/ghostllm.log"
```

### Systemd Service

**Create `/etc/systemd/system/ghostllm.service`:**
```ini
[Unit]
Description=GhostLLM Proxy Server
After=network.target postgresql.service redis.service
Wants=postgresql.service redis.service

[Service]
Type=exec
User=ghostllm
Group=ghostllm
WorkingDirectory=/var/lib/ghostllm
ExecStart=/usr/local/bin/ghostllm-proxy serve --config /etc/ghostllm/config.toml
Restart=always
RestartSec=10

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/ghostllm /var/log/ghostllm

# Resource limits
LimitNOFILE=65536
MemoryMax=4G

[Install]
WantedBy=multi-user.target
```

**Enable and start:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable ghostllm
sudo systemctl start ghostllm

# Check status
sudo systemctl status ghostllm
sudo journalctl -u ghostllm -f
```

## ☁️ Cloud Deployment

### AWS Deployment

**ECS with Fargate:**

```yaml
# ecs-task-definition.json
{
  "family": "ghostllm",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "executionRoleArn": "arn:aws:iam::account:role/ecsTaskExecutionRole",
  "containerDefinitions": [
    {
      "name": "ghostllm",
      "image": "ghostllm:latest",
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "DATABASE_URL",
          "value": "postgresql://..."
        }
      ],
      "secrets": [
        {
          "name": "OPENAI_API_KEY",
          "valueFrom": "arn:aws:secretsmanager:region:account:secret:ghostllm/openai-key"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/ghostllm",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

**Deploy with CDK:**
```typescript
import * as ecs from 'aws-cdk-lib/aws-ecs';
import * as ec2 from 'aws-cdk-lib/aws-ec2';

// Create VPC
const vpc = new ec2.Vpc(this, 'GhostLLMVpc', {
  maxAzs: 2
});

// Create ECS cluster
const cluster = new ecs.Cluster(this, 'GhostLLMCluster', {
  vpc: vpc
});

// Create Fargate service
const service = new ecs.FargateService(this, 'GhostLLMService', {
  cluster: cluster,
  taskDefinition: taskDefinition,
  desiredCount: 2
});
```

### Google Cloud Platform

**Cloud Run deployment:**
```yaml
# cloudrun.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: ghostllm
spec:
  template:
    metadata:
      annotations:
        autoscaling.knative.dev/maxScale: "10"
        run.googleapis.com/cpu-throttling: "false"
    spec:
      containerConcurrency: 100
      containers:
      - image: gcr.io/project-id/ghostllm:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ghostllm-secrets
              key: database-url
        resources:
          limits:
            cpu: "2"
            memory: "4Gi"
```

**Deploy:**
```bash
# Build and push image
gcloud builds submit --tag gcr.io/project-id/ghostllm

# Deploy to Cloud Run
gcloud run deploy ghostllm \
  --image gcr.io/project-id/ghostllm \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated
```

## 🔒 Security Hardening

### SSL/TLS Configuration

**Generate certificates:**
```bash
# Using Let's Encrypt
certbot certonly --standalone -d api.ghostllm.com

# Or generate self-signed
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

**Nginx SSL configuration:**
```nginx
server {
    listen 443 ssl http2;
    server_name api.ghostllm.com;

    ssl_certificate /etc/ssl/certs/ghostllm.pem;
    ssl_certificate_key /etc/ssl/private/ghostllm.key;

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
    ssl_prefer_server_ciphers off;

    location / {
        proxy_pass http://ghostllm:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Firewall Configuration

```bash
# UFW (Ubuntu)
ufw default deny incoming
ufw default allow outgoing
ufw allow ssh
ufw allow 80/tcp
ufw allow 443/tcp
ufw enable

# iptables
iptables -A INPUT -p tcp --dport 22 -j ACCEPT
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
iptables -A INPUT -j DROP
```

## 📊 Monitoring & Logging

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'ghostllm'
    static_configs:
      - targets: ['ghostllm:9090']
    metrics_path: /metrics
    scrape_interval: 30s
```

### Grafana Dashboard

Import dashboard ID: `ghostllm-overview.json`

Key metrics:
- Request rate and latency
- Error rates by provider
- Token usage and costs
- Active users and API keys
- System resources (CPU, memory)

### Log Aggregation

**Fluentd configuration:**
```xml
<source>
  @type tail
  path /var/log/ghostllm/*.log
  pos_file /var/log/fluentd/ghostllm.log.pos
  tag ghostllm
  format json
</source>

<match ghostllm>
  @type elasticsearch
  host elasticsearch
  port 9200
  index_name ghostllm
</match>
```

## 🔄 Backup & Recovery

### Database Backup

```bash
# Automated backup script
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups/ghostllm"

# Create backup
pg_dump $DATABASE_URL > $BACKUP_DIR/ghostllm_$DATE.sql

# Compress
gzip $BACKUP_DIR/ghostllm_$DATE.sql

# Clean old backups (keep 30 days)
find $BACKUP_DIR -name "*.sql.gz" -mtime +30 -delete

# Upload to S3 (optional)
aws s3 cp $BACKUP_DIR/ghostllm_$DATE.sql.gz s3://ghostllm-backups/
```

### Configuration Backup

```bash
# Backup critical files
tar -czf ghostllm-config-$(date +%Y%m%d).tar.gz \
  /etc/ghostllm/ \
  /var/lib/ghostllm/ \
  docker-compose.yml \
  .env
```

### Disaster Recovery

1. **Restore database:**
   ```bash
   gunzip -c backup.sql.gz | psql $DATABASE_URL
   ```

2. **Restore configuration:**
   ```bash
   tar -xzf ghostllm-config-backup.tar.gz -C /
   ```

3. **Restart services:**
   ```bash
   docker-compose up -d
   # or
   systemctl restart ghostllm
   ```

## 🚨 Troubleshooting

### Common Issues

**Health check failing:**
```bash
# Check logs
docker-compose logs ghostllm
journalctl -u ghostllm -n 50

# Verify network connectivity
curl -v http://localhost:8080/health

# Check database connection
psql $DATABASE_URL -c "SELECT 1;"
```

**High memory usage:**
```bash
# Adjust memory limits
docker-compose exec ghostllm sh -c 'echo 1 > /proc/sys/vm/drop_caches'

# Monitor with htop
htop

# Check Redis memory
redis-cli info memory
```

**Provider API errors:**
```bash
# Test provider connectivity
curl -H "Authorization: Bearer $OPENAI_API_KEY" \
  https://api.openai.com/v1/models

# Check proxy logs
docker-compose logs ghostllm | grep -i error
```

---

**For deployment assistance, check the troubleshooting guide or open an issue on GitHub.**