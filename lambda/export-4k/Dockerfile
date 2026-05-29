# Build Stage — ใช้ Amazon Linux 2023 เหมือน Lambda runtime เพื่อให้ GLIBC version ตรงกัน
FROM public.ecr.aws/amazonlinux/amazonlinux:2023 AS builder

WORKDIR /app

RUN dnf install -y openssl-devel postgresql-devel gcc make tar && dnf clean all

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy workspace files ทั้งหมดที่จำเป็น
COPY Cargo.toml Cargo.lock ./
COPY backend/shared ./backend/shared
COPY backend/api ./backend/api
COPY backend/worker ./backend/worker
COPY lambda/export-4k/Cargo.toml ./lambda/export-4k/Cargo.toml
COPY lambda/export-4k/src ./lambda/export-4k/src

RUN cargo build --release -p export-lambda

# Runtime Stage
FROM public.ecr.aws/lambda/provided:al2023

COPY --from=builder /app/target/release/bootstrap /var/runtime/bootstrap
COPY lambda/export-4k/ffmpeg /usr/local/bin/ffmpeg
COPY lambda/export-4k/ffprobe /usr/local/bin/ffprobe
RUN chmod +x /usr/local/bin/ffmpeg /usr/local/bin/ffprobe /var/runtime/bootstrap

CMD ["bootstrap"]
