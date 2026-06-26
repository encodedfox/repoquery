#!/usr/bin/env python3
"""Apply reference/architecture/tool tags to repos in SQLite — precise version."""
import json
import sqlite3
from collections import Counter
from datetime import datetime, timedelta

DB_PATH = "data/repoquery.db"
NOW = datetime.now()
ACTIVE_MONTHS = 3
STALE_MONTHS = 12

def classify_activity(last_commit_date):
    if not last_commit_date:
        return "unknown"
    try:
        d = datetime.strptime(last_commit_date, "%Y-%m-%d")
    except ValueError:
        return "unknown"
    ac = NOW - timedelta(days=ACTIVE_MONTHS * 30)
    sc = NOW - timedelta(days=STALE_MONTHS * 30)
    abc = NOW - timedelta(days=24 * 30)
    if d >= ac: return "active"
    elif d >= sc: return "maintained"
    elif d >= abc: return "stale"
    else: return "abandoned"

def classify_tag(full_name, name, desc, topics, lang, stars, health):
    text = f"{name} {desc}".lower()
    ts = {t.lower() for t in (topics or [])}
    all_text = text + " " + " ".join(ts)

    # === Reference / Documentation / List ===
    ref_score = 0
    # Strong signals (2x weight)
    if any(p in all_text for p in ["awesome", "curated list", "awesome list"]):
        ref_score += 3
    if "awesome-list" in ts or "awesome" in ts:
        ref_score += 3
    # Standard reference signals
    for p in ["reference", "guide", "tutorial", "documentation", "learning",
              "reading list", "resources", "notes", "how to", "getting started",
              "paper", "roadmap", "curriculum", "course", "cookbook", "handbook",
              "cheat sheet", "examples of", "list of"]:
        if p in all_text:
            ref_score += 1
    for t in {"documentation", "tutorial", "educational", "learning",
              "reference", "paper", "resources", "guide", "roadmap", "curriculum"}:
        if t in ts:
            ref_score += 2

    if ref_score >= 2:
        return "ref:reference" if health else "ref:archived-ref"

    # === Architecture / Design / Spec ===
    arch_score = 0
    for p in ["architecture", "design pattern", "architectural",
              "specification", "standard", "protocol", "schema",
              "blueprint", "template", "clean architecture"]:
        if p in all_text:
            arch_score += 1
    for t in {"architecture", "design", "pattern", "design-pattern",
              "best-practice", "template", "specification"}:
        if t in ts:
            arch_score += 2

    if arch_score >= 2:
        return "ref:architecture" if health else "ref:archived-arch"

    # === Tool / Library / Framework (only tag non-healthy) ===
    if not health:
        tool_score = 0
        for p in ["framework", "library", "sdk", "toolkit", "engine",
                  "cli ", "compiler", "interpreter", "runtime", "database ",
                  "cache ", "queue ", "server ", "client library"]:
            if p in all_text:
                tool_score += 1
        for t in {"framework", "library", "sdk", "cli", "tool",
                  "database", "cache", "compiler", "interpreter",
                  "runtime", "engine"}:
            if t in ts:
                tool_score += 2
        if tool_score >= 2 or stars >= 500:
            return "ref:archived-tool"

    return None

def main():
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    cursor.execute("SELECT * FROM repositories")
    rows = cursor.fetchall()

    counts = Counter()
    updates = []

    for row in rows:
        meta = json.loads(row["metadata_json"])
        qual = json.loads(row["quality_json"])
        cur_tags = json.loads(row["custom_tags_json"])

        last_commit = qual.get("last_commit_date") or qual.get("last_star_update")
        activity = classify_activity(last_commit)
        health = activity in ("active", "maintained")

        full_name = row["full_name"]
        name = row["name"]
        desc = meta.get("description") or ""
        topics = meta.get("topics") or []
        stars = row["stars"]
        lang = row["primary_language"] or "Unknown"
        rid = row["id"]

        tag = classify_tag(full_name, name, desc, topics, lang, stars, health)

        if tag and tag not in cur_tags:
            cur_tags.append(tag)
            counts["tagged"] += 1
        elif tag:
            counts["already_tagged"] += 1
        else:
            counts["no_tag"] += 1

        updates.append((json.dumps(cur_tags), rid))

    cursor.executemany(
        "UPDATE repositories SET custom_tags_json = ? WHERE id = ?",
        updates
    )
    conn.commit()

    print(f"Tagged:         {counts['tagged']}")
    print(f"Already tagged: {counts['already_tagged']}")
    print(f"No tag:         {counts['no_tag']}")
    print(f"Total:          {sum(counts.values())}")

    conn.close()

if __name__ == "__main__":
    main()
