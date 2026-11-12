# Agent-One — DropAgentX v3 server (فاز ۴ از ADR-009)
# بیلد دومرحله‌ای: builder (rust:slim) → runtime (distroless) — تک‌باینری.
#
#   docker build -t dropagentx-server .
#   docker run -p 3000:3000 -e KIA_ADMIN_KEY=... -e DATABASE_URL=file:/data/v3.db \
#     -v agent-data:/data dropagentx-server
#
# assets داشبورد (front/dashboard/dist) در فاز ۷ با ادغام فرانت COPY می‌شود؛
# مسیر آن با STATIC_DIR به باینری معرفی می‌شود (پیش‌فرض front/dashboard/dist).

# ── مرحلهٔ ۱ — builder ──────────────────────────────────────────────────────
FROM rust:1.99-slim AS builder
WORKDIR /build

# کش وابستگی‌ها — فقط manifest ها اول کپی می‌شوند
COPY Cargo.toml Cargo.lock rust-toolchain.toml clippy.toml ./
COPY crates ./crates

RUN cargo build --release -p dropagentx-server

# ── مرحلهٔ ۲ — runtime ──────────────────────────────────────────────────────
# distroless/cc — glibc + libgcc برای SQLite باندل‌شده؛ شل، بدون پکیج‌منیجر،
# بدون supply-chain اضافه. کاربر غیر-root.
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app

# تک‌باینری — همهٔ assetsها (مهاجرت‌های sqlx) در کامپایل جاسازی شده‌اند
COPY --from=builder /build/target/release/dropagentx-server /app/dropagentx-server

ENV HOST=0.0.0.0 \
    PORT=3000 \
    DATABASE_URL=file:/data/v3.db \
    STATIC_DIR=/app/dashboard

EXPOSE 3000
VOLUME ["/data"]

ENTRYPOINT ["/app/dropagentx-server"]
