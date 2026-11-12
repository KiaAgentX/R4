#!/usr/bin/env python3
"""سینتیک legacy fixture با «دقیقاً» اعداد روز-اول (عین build_legacy_fixture در
integration_tests.rs فاز ۲) — برای تمرین cut-over فاز ۸:

  465 × 403.0 + 1 × 277.8 = 187,672.8 credits  →  Σ floor = 187,672 Đ

products: 30 ردیف · transactions: 1,554 ردیف — همان شکل بکاپ Railway
(marketplace-20260924-0400.db).
"""
import sqlite3
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else "/tmp/legacy/marketplace-20260924-0400.db"
USERS_N = 466
PRODUCTS_N = 30
TX_N = 1_554

conn = sqlite3.connect(OUT)
cur = conn.cursor()
cur.execute("CREATE TABLE users (user_id INTEGER PRIMARY KEY, username TEXT, first_name TEXT, credits REAL, is_banned INTEGER)")
cur.execute("CREATE TABLE products (id INTEGER PRIMARY KEY, creator_id INTEGER, title TEXT, description TEXT, price_credits REAL, category TEXT, link TEXT, is_active INTEGER)")
cur.execute("CREATE TABLE transactions (id INTEGER PRIMARY KEY, note TEXT)")

total = 0.0
for i in range(USERS_N):
    credits = 277.8 if i + 1 == USERS_N else 403.0
    total += credits
    cur.execute("INSERT INTO users VALUES (?,?,?,?,?)",
                (501_070_000 + i, f"user{i:03}", f"شهروند {i}", credits, 1 if i % 100 == 0 else 0))

for i in range(PRODUCTS_N):
    cur.execute("INSERT INTO products VALUES (?,?,?,?,?,?,?,?)",
                (i + 1, 501_070_000 + i % USERS_N,
                 f"عنوان محصول {i} — " + "ز" * 150, f"توضیح محصول {i}",
                 49.9 if i % 2 == 0 else 120.0, f"cat-{i % 5}",
                 f"https://dl.example/{i}" if i % 3 == 0 else "", 1 if i % 4 == 0 else 0))

for i in range(TX_N):
    cur.execute("INSERT INTO transactions (note) VALUES (?)", (f"legacy tx {i}",))

conn.commit()
conn.close()
print(f"legacy fixture → {OUT}")
print(f"users = {USERS_N} · Σ credits = {total:.1f} (floor → {int(total)} Đ)")
print(f"products = {PRODUCTS_N} · transactions = {TX_N}")
