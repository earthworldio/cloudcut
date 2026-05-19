# 4. Copy env
cp .env.example .env

# 5. Build จาก source
docker compose build

# 6. Start infra ก่อน (db, redis, minio, minio-init)
docker compose up -d db redis minio minio-init

# 7. รอ minio-init เสร็จ แล้วรัน migrations
docker compose logs -f minio-init  # รอจนเห็น "Bucket created" แล้ว Ctrl+C

# 8. รัน migrations
docker exec $(docker compose ps -q db) \
  sh -c "PGPASSWORD=password psql -U user cloudcut" < backend/migrations/0001_init.sql
docker exec $(docker compose ps -q db) \
  sh -c "PGPASSWORD=password psql -U user cloudcut" < backend/migrations/0002_indexes.sql
docker exec $(docker compose ps -q db) \
  sh -c "PGPASSWORD=password psql -U user cloudcut" < backend/migrations/0003_seed.sql

# 9. Start ทุกอย่าง
docker compose up -d


เข้าได้ที่ http://localhost 

login: alice@cloudcut.com / password123