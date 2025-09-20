# GhostFlow Deployment Guide

Complete guide for deploying GhostFlow in various environments.

## 🚀 Quick Deployment Options

### Option 1: Docker Compose (Recommended)

```bash
# Clone and deploy
git clone https://github.com/ghostkellz/ghostflow
cd ghostflow
./scripts/start.sh prod

# Services will be available at:
# UI: http://localhost:8080
# API: http://localhost:3000
```

### Option 2: One-Command Deploy

```bash
curl -fsSL https://raw.githubusercontent.com/ghostkellz/ghostflow/main/scripts/install.sh | bash
```

---

## 🐳 Docker Deployment

### Production Docker Compose

Create `docker-compose.prod.yml`:

```yaml
version: '3.8'

services:
  ghostflow:
    image: ghostflow/ghostflow:latest
    restart: unless-stopped
    ports:
      - "3000:3000"
      - "8080:8080"
    environment:
      DATABASE_URL: postgresql://ghostflow:${POSTGRES_PASSWORD}@postgres/ghostflow
      MINIO_ENDPOINT: http://minio:9000
      MINIO_ACCESS_KEY: ${MINIO_ACCESS_KEY}
      MINIO_SECRET_KEY: ${MINIO_SECRET_KEY}
      OLLAMA_HOST: http://ollama:11434
      RUST_LOG: info
      JWT_SECRET: ${JWT_SECRET}
    depends_on:
      - postgres
      - minio
    volumes:
      - ./config:/app/config:ro
      - ghostflow_data:/app/data
    networks:
      - ghostflow
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres:
    image: postgres:16-alpine
    restart: unless-stopped
    environment:
      POSTGRES_USER: ghostflow
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ghostflow
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./backups:/backups
    networks:
      - ghostflow
    ports:
      - "127.0.0.1:5432:5432"  # Only bind to localhost
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ghostflow"]
      interval: 10s
      timeout: 5s
      retries: 5

  minio:
    image: minio/minio:latest
    restart: unless-stopped
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: ${MINIO_ACCESS_KEY}
      MINIO_ROOT_PASSWORD: ${MINIO_SECRET_KEY}
    volumes:
      - minio_data:/data
    networks:
      - ghostflow
    ports:
      - "127.0.0.1:9001:9001"  # Console access
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 30s
      timeout: 10s
      retries: 3

  ollama:
    image: ollama/ollama:latest
    restart: unless-stopped
    volumes:
      - ollama_data:/root/.ollama
    networks:
      - ghostflow
    ports:
      - "127.0.0.1:11434:11434"
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: all
              capabilities: [gpu]
    # For CPU-only deployment, remove the deploy section

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
      - nginx_logs:/var/log/nginx
    depends_on:
      - ghostflow
    networks:
      - ghostflow

volumes:
  postgres_data:
    driver: local
  minio_data:
    driver: local
  ollama_data:
    driver: local
  ghostflow_data:
    driver: local
  nginx_logs:
    driver: local

networks:
  ghostflow:
    driver: bridge
```

### Environment Configuration

Create `.env` file:

```bash
# Database
POSTGRES_PASSWORD=your_secure_password_here

# MinIO S3 Storage
MINIO_ACCESS_KEY=ghostflow_access
MINIO_SECRET_KEY=your_minio_secret_key_here

# JWT Authentication
JWT_SECRET=your_jwt_secret_here_make_it_long_and_random

# Optional: Backup settings
BACKUP_SCHEDULE="0 2 * * *"  # Daily at 2 AM
BACKUP_RETENTION_DAYS=30

# Optional: Monitoring
PROMETHEUS_ENABLED=true
GRAFANA_ADMIN_PASSWORD=admin_password_here
```

### Nginx Configuration

Create `nginx.conf`:

```nginx
events {
    worker_connections 1024;
}

http {
    upstream ghostflow_api {
        server ghostflow:3000;
    }

    upstream ghostflow_ui {
        server ghostflow:8080;
    }

    server {
        listen 80;
        server_name your-domain.com;
        return 301 https://$server_name$request_uri;
    }

    server {
        listen 443 ssl http2;
        server_name your-domain.com;

        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;

        # API routes
        location /api/ {
            proxy_pass http://ghostflow_api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # WebSocket
        location /ws {
            proxy_pass http://ghostflow_api;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # UI routes
        location / {
            proxy_pass http://ghostflow_ui;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # Security headers
        add_header X-Frame-Options DENY;
        add_header X-Content-Type-Options nosniff;
        add_header X-XSS-Protection "1; mode=block";
        add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload";
    }
}
```

### Deploy Command

```bash
# Deploy production stack
docker-compose -f docker-compose.prod.yml up -d

# View logs
docker-compose -f docker-compose.prod.yml logs -f

# Update
docker-compose -f docker-compose.prod.yml pull
docker-compose -f docker-compose.prod.yml up -d --no-deps ghostflow
```

---

## ☸️ Kubernetes Deployment

### Namespace and ConfigMap

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: ghostflow

---
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: ghostflow-config
  namespace: ghostflow
data:
  RUST_LOG: "info"
  OLLAMA_HOST: "http://ollama:11434"
  MINIO_ENDPOINT: "http://minio:9000"
```

### Secrets

```yaml
# k8s/secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: ghostflow-secrets
  namespace: ghostflow
type: Opaque
data:
  database-url: cG9zdGdyZXNxbDovL2dob3N0ZmxvdzpwYXNzd29yZEBwb3N0Z3Jlcy9naG9zdGZsb3c=  # base64 encoded
  jwt-secret: eW91cl9qd3Rfc2VjcmV0X2hlcmU=  # base64 encoded
  minio-access-key: Z2hvc3RmbG93X2FjY2Vzcw==  # base64 encoded
  minio-secret-key: eW91cl9taW5pb19zZWNyZXRfa2V5  # base64 encoded
```

### PostgreSQL

```yaml
# k8s/postgres.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: postgres
  namespace: ghostflow
spec:
  replicas: 1
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
      - name: postgres
        image: postgres:16-alpine
        env:
        - name: POSTGRES_USER
          value: "ghostflow"
        - name: POSTGRES_PASSWORD
          valueFrom:
            secretKeyRef:
              name: ghostflow-secrets
              key: postgres-password
        - name: POSTGRES_DB
          value: "ghostflow"
        ports:
        - containerPort: 5432
        volumeMounts:
        - name: postgres-storage
          mountPath: /var/lib/postgresql/data
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
      volumes:
      - name: postgres-storage
        persistentVolumeClaim:
          claimName: postgres-pvc

---
apiVersion: v1
kind: Service
metadata:
  name: postgres
  namespace: ghostflow
spec:
  selector:
    app: postgres
  ports:
  - port: 5432
    targetPort: 5432

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: postgres-pvc
  namespace: ghostflow
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
```

### GhostFlow Application

```yaml
# k8s/ghostflow.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ghostflow
  namespace: ghostflow
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ghostflow
  template:
    metadata:
      labels:
        app: ghostflow
    spec:
      containers:
      - name: ghostflow
        image: ghostflow/ghostflow:latest
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ghostflow-secrets
              key: database-url
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: ghostflow-secrets
              key: jwt-secret
        - name: MINIO_ACCESS_KEY
          valueFrom:
            secretKeyRef:
              name: ghostflow-secrets
              key: minio-access-key
        - name: MINIO_SECRET_KEY
          valueFrom:
            secretKeyRef:
              name: ghostflow-secrets
              key: minio-secret-key
        envFrom:
        - configMapRef:
            name: ghostflow-config
        ports:
        - containerPort: 3000
          name: api
        - containerPort: 8080
          name: ui
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5

---
apiVersion: v1
kind: Service
metadata:
  name: ghostflow-service
  namespace: ghostflow
spec:
  selector:
    app: ghostflow
  ports:
  - name: api
    port: 3000
    targetPort: 3000
  - name: ui
    port: 8080
    targetPort: 8080
  type: ClusterIP

---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: ghostflow-ingress
  namespace: ghostflow
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - ghostflow.your-domain.com
    secretName: ghostflow-tls
  rules:
  - host: ghostflow.your-domain.com
    http:
      paths:
      - path: /api
        pathType: Prefix
        backend:
          service:
            name: ghostflow-service
            port:
              number: 3000
      - path: /ws
        pathType: Exact
        backend:
          service:
            name: ghostflow-service
            port:
              number: 3000
      - path: /
        pathType: Prefix
        backend:
          service:
            name: ghostflow-service
            port:
              number: 8080
```

### Horizontal Pod Autoscaler

```yaml
# k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: ghostflow-hpa
  namespace: ghostflow
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: ghostflow
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

### Deploy to Kubernetes

```bash
# Apply all configurations
kubectl apply -f k8s/

# Check status
kubectl get pods -n ghostflow

# View logs
kubectl logs -f deployment/ghostflow -n ghostflow

# Port forward for local testing
kubectl port-forward service/ghostflow-service 3000:3000 8080:8080 -n ghostflow
```

---

## 🏠 Bare Metal Deployment

### System Requirements

**Minimum:**
- 2 CPU cores
- 4 GB RAM
- 20 GB storage
- Ubuntu 20.04+ / CentOS 8+ / RHEL 8+

**Recommended:**
- 4+ CPU cores
- 8+ GB RAM
- 100+ GB SSD storage
- Load balancer (nginx/haproxy)

### Installation Script

Create `install.sh`:

```bash
#!/bin/bash
set -e

echo "🚀 Installing GhostFlow..."

# Install dependencies
if command -v apt-get >/dev/null 2>&1; then
    # Ubuntu/Debian
    sudo apt-get update
    sudo apt-get install -y curl wget build-essential pkg-config libssl-dev
elif command -v yum >/dev/null 2>&1; then
    # CentOS/RHEL
    sudo yum groupinstall -y "Development Tools"
    sudo yum install -y curl wget openssl-devel
fi

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Install PostgreSQL
if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get install -y postgresql postgresql-contrib
elif command -v yum >/dev/null 2>&1; then
    sudo yum install -y postgresql-server postgresql-contrib
    sudo postgresql-setup initdb
fi

sudo systemctl enable postgresql
sudo systemctl start postgresql

# Create database and user
sudo -u postgres psql -c "CREATE DATABASE ghostflow;"
sudo -u postgres psql -c "CREATE USER ghostflow WITH PASSWORD 'ghostflow';"
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE ghostflow TO ghostflow;"

# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Clone and build GhostFlow
git clone https://github.com/ghostkellz/ghostflow
cd ghostflow
export DATABASE_URL="postgresql://ghostflow:ghostflow@localhost/ghostflow"
sqlx migrate run
cargo build --release

# Install binaries
sudo cp target/release/ghostflow-server /usr/local/bin/
sudo cp target/release/gflow /usr/local/bin/
sudo chmod +x /usr/local/bin/ghostflow-*
sudo chmod +x /usr/local/bin/gflow

# Create systemd service
sudo tee /etc/systemd/system/ghostflow.service > /dev/null <<EOF
[Unit]
Description=GhostFlow Server
After=network.target postgresql.service

[Service]
Type=simple
User=ghostflow
Group=ghostflow
WorkingDirectory=/opt/ghostflow
Environment=DATABASE_URL=postgresql://ghostflow:ghostflow@localhost/ghostflow
Environment=RUST_LOG=info
ExecStart=/usr/local/bin/ghostflow-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Create user and directories
sudo useradd -r -s /bin/false ghostflow
sudo mkdir -p /opt/ghostflow/{config,data,logs}
sudo chown -R ghostflow:ghostflow /opt/ghostflow

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable ghostflow
sudo systemctl start ghostflow

echo "✅ GhostFlow installed successfully!"
echo "   API: http://localhost:3000"
echo "   Status: sudo systemctl status ghostflow"
echo "   Logs: sudo journalctl -u ghostflow -f"
```

### Service Management

```bash
# Start/stop/restart
sudo systemctl start ghostflow
sudo systemctl stop ghostflow
sudo systemctl restart ghostflow

# Enable/disable auto-start
sudo systemctl enable ghostflow
sudo systemctl disable ghostflow

# Check status
sudo systemctl status ghostflow

# View logs
sudo journalctl -u ghostflow -f
sudo journalctl -u ghostflow --since "1 hour ago"
```

### Nginx Proxy Configuration

```bash
# Install nginx
sudo apt-get install nginx

# Configure
sudo tee /etc/nginx/sites-available/ghostflow > /dev/null <<EOF
server {
    listen 80;
    server_name your-domain.com;

    location /api/ {
        proxy_pass http://localhost:3000;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }

    location /ws {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
    }

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host \$host;
    }
}
EOF

# Enable site
sudo ln -s /etc/nginx/sites-available/ghostflow /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

---

## 📊 Monitoring

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'ghostflow'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: '/metrics'
    scrape_interval: 5s

  - job_name: 'postgres'
    static_configs:
      - targets: ['localhost:9187']

  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']
```

### Grafana Dashboard

Import dashboard ID: `15760` or create custom dashboard with these queries:

```promql
# Flow execution rate
rate(ghostflow_executions_total[5m])

# Average execution time
avg(ghostflow_execution_duration_seconds)

# Error rate
rate(ghostflow_errors_total[5m])

# Active flows
ghostflow_active_flows

# Database connections
postgres_stat_database_numbackends
```

### Log Aggregation

**With ELK Stack:**

```yaml
# filebeat.yml
filebeat.inputs:
- type: log
  enabled: true
  paths:
    - /var/log/ghostflow/*.log
  fields:
    service: ghostflow
  fields_under_root: true

output.elasticsearch:
  hosts: ["localhost:9200"]
  index: "ghostflow-logs-%{+yyyy.MM.dd}"
```

**With Loki:**

```yaml
# promtail.yml
server:
  http_listen_port: 9080

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://localhost:3100/loki/api/v1/push

scrape_configs:
- job_name: ghostflow
  static_configs:
  - targets:
      - localhost
    labels:
      job: ghostflow
      __path__: /var/log/ghostflow/*.log
```

---

## 🔐 Security

### SSL/TLS Configuration

```bash
# Get Let's Encrypt certificate
sudo apt-get install certbot python3-certbot-nginx
sudo certbot --nginx -d your-domain.com
```

### Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw allow ssh
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw --force enable

# firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload
```

### Security Headers

Add to nginx configuration:

```nginx
# Security headers
add_header X-Frame-Options DENY;
add_header X-Content-Type-Options nosniff;
add_header X-XSS-Protection "1; mode=block";
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload";
add_header Referrer-Policy "strict-origin-when-cross-origin";
add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline';";
```

---

## 🔄 Backup and Recovery

### Database Backup

```bash
#!/bin/bash
# backup.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/opt/backups/ghostflow"
DB_NAME="ghostflow"

mkdir -p $BACKUP_DIR

# Database backup
pg_dump -h localhost -U ghostflow -d $DB_NAME | gzip > $BACKUP_DIR/db_$DATE.sql.gz

# Data directory backup
tar -czf $BACKUP_DIR/data_$DATE.tar.gz /opt/ghostflow/data

# Clean old backups (keep 30 days)
find $BACKUP_DIR -name "*.gz" -mtime +30 -delete

echo "Backup completed: $BACKUP_DIR/db_$DATE.sql.gz"
```

### Automated Backups

```bash
# Add to crontab
crontab -e

# Daily backup at 2 AM
0 2 * * * /opt/ghostflow/scripts/backup.sh

# Weekly full backup
0 3 * * 0 /opt/ghostflow/scripts/full-backup.sh
```

### Recovery

```bash
# Restore database
gunzip -c /opt/backups/ghostflow/db_20240108_020000.sql.gz | psql -h localhost -U ghostflow -d ghostflow

# Restore data
sudo systemctl stop ghostflow
tar -xzf /opt/backups/ghostflow/data_20240108_020000.tar.gz -C /
sudo chown -R ghostflow:ghostflow /opt/ghostflow/data
sudo systemctl start ghostflow
```

---

## 🔧 Troubleshooting

### Common Issues

**1. Database Connection Failed**
```bash
# Check PostgreSQL status
sudo systemctl status postgresql

# Check connection
psql -h localhost -U ghostflow -d ghostflow -c "SELECT 1;"

# Check logs
sudo journalctl -u postgresql -f
```

**2. Port Already in Use**
```bash
# Find process using port
sudo netstat -tulpn | grep :3000
sudo kill -9 <PID>
```

**3. Permission Denied**
```bash
# Fix ownership
sudo chown -R ghostflow:ghostflow /opt/ghostflow
sudo chmod -R 755 /opt/ghostflow
```

**4. Out of Memory**
```bash
# Check memory usage
free -h
top -p $(pgrep ghostflow-server)

# Adjust systemd service
sudo systemctl edit ghostflow

# Add:
[Service]
MemoryLimit=1G
CPUQuota=50%
```

### Performance Tuning

**PostgreSQL:**
```sql
-- In postgresql.conf
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 4MB
maintenance_work_mem = 64MB
max_connections = 100
```

**GhostFlow:**
```bash
# Environment variables
export TOKIO_WORKER_THREADS=4
export RAYON_NUM_THREADS=4
export DATABASE_MAX_CONNECTIONS=20
```

### Health Checks

```bash
#!/bin/bash
# health-check.sh

# API health
curl -f http://localhost:3000/health || exit 1

# Database health
psql -h localhost -U ghostflow -d ghostflow -c "SELECT 1;" || exit 2

# Disk space
df -h /opt/ghostflow | awk 'NR==2 {if($5+0 > 90) exit 3}'

echo "All health checks passed"
```

---

## 📈 Scaling

### Horizontal Scaling

1. **Load Balancer Configuration**
2. **Shared Database**
3. **Distributed File Storage**
4. **Session Stickiness**

### Vertical Scaling

1. **Increase CPU/Memory**
2. **Optimize Database**
3. **Tune Worker Threads**
4. **Profile and Optimize**

---

*Deployment guide version 1.0 - Last updated: 2025-01-08*