# Agent-One — تصویر production کامل (بک‌اند Rust + استاتیک ترکیبی فاز ۷)
#
# برخلاف rust/Dockerfile (که فقط باینری سرور را می‌سازد)، این تصویرِ
# «محصول نهایی» است: static-root ترکیبی (shell + dashboard + webapp) در
# همان ایمیج ساخته و سرو می‌شود — یک کانتینر، یک متغیر صفر.
#
#   docker build -t dropagentx-production .
#   docker run -p 3000:3000 -e KIA_ADMIN_KEY=... -e DATABASE_URL=file:/data/v3.db \
#     -v agent-data:/data dropagentx-production
#
# روی Railway: Root Directory = ریشهٔ ریپو — Railway خودش همین Dockerfile را
# می‌سازد (railway.toml) و فقط متغیرها + Volume می‌خواهد.

# ── مرحلهٔ ۰ — استاتیک ترکیبی (فاز ۷) ───────────────────────────────────────
FROM node:24-slim AS static
WORKDIR /front
COPY front/package.json front/pnpm-lock.yaml front/pnpm-workspace.yaml front/env.d.ts ./
COPY front/apps ./apps
COPY front/packages ./packages
COPY front/scripts ./scripts
RUN corepack enable \
 && pnpm install --frozen-lockfile \
 && pnpm build:root
# خروجی: /front/dist  (index.html شل در ریشه · admin/ · app/ · build-manifest.json)

# ── مرحلهٔ ۱ — builder رست ──────────────────────────────────────────────────
FROM rust:1.99-slim AS builder
WORKDIR /build
COPY rust/Cargo.toml rust/Cargo.lock rust/rust-toolchain.toml rust/clippy.toml ./
COPY rust/crates ./crates
RUN cargo build --release -p dropagentx-server

# ── مرحلهٔ ۲ — runtime (distroless، غیر-root) ───────────────────────────────
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app

COPY --from=builder /build/target/release/dropagentx-server /app/dropagentx-server
COPY --from=static /front/dist /app/dashboard

ENV HOST=0.0.0.0 \
    PORT=3000 \
    DATABASE_URL=file:/data/v3.db \
    STATIC_DIR=/app/dashboard

EXPOSE 3000
VOLUME ["/data"]

ENTRYPOINT ["/app/dropagentx-server"]
