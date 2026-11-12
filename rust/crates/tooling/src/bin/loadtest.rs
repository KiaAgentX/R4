//! `loadtest` — لودتست کوتاه فاز ۸ (ADR-009 §۷ فاز ۸): «p95 زیر TS baseline
//! در همان سخت‌افزار + حافظهٔ مقصد ثبت‌شده».
//!
//! مولد بار tokio — معادل سبک `oha`: N کارگر هم‌زمان، M ثانیه، شمارش تأخیرها
//! و محاسبهٔ p50/p95/p99 و RPS و توزیع وضعیت. خروجی JSON با `--json` برای
//! ثبت در سند مقایسه (LOADTEST.md).
//!
//! استفاده:
//! ```text
//! loadtest --url http://127.0.0.1:3000/api/v1/health \
//!          [--duration 10] [--concurrency 8] [--header "x-kia-key: ..."] \
//!          [--body '{}'] [--json out.json] [--label health]
//! ```

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// خواندن مقدار `--flag` از آرگومان‌ها.
fn arg_of(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(url) = arg_of(&args, "--url") else {
        eprintln!("usage: loadtest --url <u> [--duration 10] [--concurrency 8] [--header k:v] [--body s] [--json f] [--label l]");
        std::process::exit(2);
    };
    let duration: u64 = arg_of(&args, "--duration").and_then(|v| v.parse().ok()).unwrap_or(10);
    let concurrency: usize =
        arg_of(&args, "--concurrency").and_then(|v| v.parse().ok()).unwrap_or(8);
    let header = arg_of(&args, "--header");
    let body = arg_of(&args, "--body");
    let json_out = arg_of(&args, "--json");
    let label = arg_of(&args, "--label").unwrap_or_else(|| "unlabeled".into());

    let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build()?;
    let stop = Arc::new(AtomicBool::new(false));
    let ok_count = Arc::new(AtomicU64::new(0));
    let err_count = Arc::new(AtomicU64::new(0));
    let latencies: Vec<u128> = Vec::new();
    let latencies = Arc::new(tokio::sync::Mutex::new(latencies));
    let status_counts: std::collections::BTreeMap<u16, u64> = Default::default();
    let status_counts = Arc::new(tokio::sync::Mutex::new(status_counts));

    let started = Instant::now();
    let mut workers = tokio::task::JoinSet::new();
    for _ in 0..concurrency {
        let client = client.clone();
        let url = url.clone();
        let stop = stop.clone();
        let ok = ok_count.clone();
        let err = err_count.clone();
        let lat = latencies.clone();
        let st = status_counts.clone();
        let header = header.clone();
        let body = body.clone();
        workers.spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                let method =
                    if body.is_some() { reqwest::Method::POST } else { reqwest::Method::GET };
                let mut req = client.request(method, &url);
                if let Some(h) = &header {
                    if let Some((k, v)) = h.split_once(':') {
                        req = req.header(k.trim(), v.trim());
                    }
                }
                if let Some(b) = &body {
                    req = req.header("content-type", "application/json").body(b.clone());
                }
                let t0 = Instant::now();
                match req.send().await {
                    Ok(res) => {
                        let status = res.status().as_u16();
                        let _ = res.bytes().await;
                        ok.fetch_add(1, Ordering::Relaxed);
                        lat.lock().await.push(t0.elapsed().as_micros());
                        st.lock().await.entry(status).and_modify(|c| *c += 1).or_insert(1);
                    }
                    Err(_) => {
                        err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });
    }
    tokio::time::sleep(Duration::from_secs(duration)).await;
    stop.store(true, Ordering::Relaxed);
    while workers.join_next().await.is_some() {}
    let elapsed = started.elapsed().as_secs_f64();

    let mut lats = latencies.lock().await.clone();
    lats.sort_unstable();
    let total = lats.len();
    let percentile = |p: f64| -> f64 {
        if lats.is_empty() {
            return 0.0;
        }
        let idx = ((p / 100.0) * (lats.len() as f64 - 1.0)).round() as usize;
        lats[idx.min(lats.len() - 1)] as f64 / 1000.0
    };
    let rps = if elapsed > 0.0 {
        f64::from(u32::try_from(ok_count.load(Ordering::Relaxed)).unwrap_or(u32::MAX)) / elapsed
    } else {
        0.0
    };
    let statuses = status_counts.lock().await.clone();

    println!("══════════ loadtest — {label} ══════════");
    println!("url          : {url}");
    println!("duration     : {elapsed:.1}s × {concurrency} workers");
    println!("requests ok  : {}", ok_count.load(Ordering::Relaxed));
    println!("errors       : {}", err_count.load(Ordering::Relaxed));
    println!("throughput   : {rps:.0} req/s");
    println!("latency p50  : {:.1} ms", percentile(50.0));
    println!("latency p95  : {:.1} ms", percentile(95.0));
    println!("latency p99  : {:.1} ms", percentile(99.0));
    for (status, count) in &statuses {
        println!("status {status}  : {count}");
    }

    if let Some(path) = json_out {
        let doc = serde_json::json!({
            "label": label,
            "url": url,
            "durationSecs": elapsed,
            "concurrency": concurrency,
            "ok": ok_count.load(Ordering::Relaxed),
            "errors": err_count.load(Ordering::Relaxed),
            "rps": rps,
            "p50ms": percentile(50.0),
            "p95ms": percentile(95.0),
            "p99ms": percentile(99.0),
            "statuses": statuses,
            "totalSamples": total,
        });
        std::fs::write(&path, serde_json::to_vec_pretty(&doc)?)?;
        println!("report → {path}");
    }
    Ok(())
}
