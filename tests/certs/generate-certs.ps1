# 一键生成 TLS 测试证书
# 用法: .\generate-certs.ps1

$certDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:OPENSSL_CONF = "$certDir\openssl.cnf"

Write-Host "=== 生成 TLS 测试证书 ===" -ForegroundColor Cyan
Write-Host "目录: $certDir" -ForegroundColor Gray

# 创建最小 OpenSSL 配置
@"
[req]
distinguished_name = req_distinguished_name
prompt = no
x509_extensions = v3_ca

[req_distinguished_name]

[v3_ca]
basicConstraints = critical,CA:TRUE
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash

[CA_default]
default_md = sha256
"@ | Out-File -Encoding ascii "$certDir\openssl.cnf"

# CA 根证书
Write-Host "[1/4] 生成 CA 根证书..." -ForegroundColor Yellow
openssl genrsa -out "$certDir\ca.key" 2048 2>$null
openssl req -x509 -new -nodes -key "$certDir\ca.key" -sha256 -days 3650 -out "$certDir\ca.pem" -subj "/CN=TLS Test CA" -config "$certDir\openssl.cnf" -extensions v3_ca

# 错误的 CA（用于负面测试）
Write-Host "[2/4] 生成错误 CA 证书..." -ForegroundColor Yellow
openssl genrsa -out "$certDir\ca-wrong.key" 2048 2>$null
openssl req -x509 -new -nodes -key "$certDir\ca-wrong.key" -sha256 -days 3650 -out "$certDir\ca-wrong.pem" -subj "/CN=Wrong CA" -config "$certDir\openssl.cnf" -extensions v3_ca

# 服务器证书
Write-Host "[3/4] 生成服务器证书..." -ForegroundColor Yellow
openssl genrsa -out "$certDir\server.key" 2048 2>$null
openssl req -new -key "$certDir\server.key" -out "$certDir\server.csr" -subj "/CN=localhost" -config "$certDir\openssl.cnf"

@"
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage=digitalSignature,nonRepudiation,keyEncipherment,dataEncipherment
subjectAltName=@alt_names
[alt_names]
DNS.1=localhost
DNS.2=127.0.0.1
IP.1=127.0.0.1
"@ | Out-File -Encoding ascii "$certDir\server.ext"

openssl x509 -req -in "$certDir\server.csr" -CA "$certDir\ca.pem" -CAkey "$certDir\ca.key" -CAcreateserial -out "$certDir\server.pem" -days 365 -sha256 -extfile "$certDir\server.ext"

# 客户端证书
Write-Host "[4/4] 生成客户端证书..." -ForegroundColor Yellow
openssl genrsa -out "$certDir\client.key" 2048 2>$null
openssl req -new -key "$certDir\client.key" -out "$certDir\client.csr" -subj "/CN=test-client" -config "$certDir\openssl.cnf"
openssl x509 -req -in "$certDir\client.csr" -CA "$certDir\ca.pem" -CAkey "$certDir\ca.key" -CAcreateserial -out "$certDir\client.pem" -days 365 -sha256

# 清理临时文件
Remove-Item "$certDir\*.csr" -ErrorAction SilentlyContinue
Remove-Item "$certDir\*.srl" -ErrorAction SilentlyContinue
Remove-Item "$certDir\server.ext" -ErrorAction SilentlyContinue

# 验证
Write-Host "`n=== 验证证书链 ===" -ForegroundColor Cyan
openssl verify -CAfile "$certDir\ca.pem" "$certDir\server.pem"
openssl verify -CAfile "$certDir\ca.pem" "$certDir\client.pem"

Write-Host "`n=== 错误 CA 验证（预期失败）===" -ForegroundColor Yellow
openssl verify -CAfile "$certDir\ca-wrong.pem" "$certDir\server.pem" 2>&1

Write-Host "`n=== 生成完毕 ===" -ForegroundColor Green
Get-ChildItem $certDir | Where-Object { -not $_.PSIsContainer -and $_.Name -notlike "*.csr" -and $_.Name -notlike "*.srl" -and $_.Name -notlike "*.ext" -and $_.Name -notlike "*.cnf" -and $_.Name -notlike "*.ps1" } | Select-Object Name, Length | Format-Table -AutoSize
