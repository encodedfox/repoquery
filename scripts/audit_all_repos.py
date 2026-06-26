#!/usr/bin/env python3
"""Comprehensive audit: classify ALL repos by type and cross-reference with prior analyses."""
import json
import sqlite3
from collections import Counter, defaultdict
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
    active_cutoff = NOW - timedelta(days=ACTIVE_MONTHS * 30)
    stale_cutoff = NOW - timedelta(days=STALE_MONTHS * 30)
    abandoned_cutoff = NOW - timedelta(days=24 * 30)
    if d >= active_cutoff:
        return "active"
    elif d >= stale_cutoff:
        return "maintained"
    elif d >= abandoned_cutoff:
        return "stale"
    else:
        return "abandoned"

# Comprehensive type classification
def classify_repo_type(name, description, topics, language, stars, homepage):
    text = f"{name} {description}".lower()
    topic_set = {t.lower() for t in (topics or [])}

    # === REFERENCE / DOCUMENTATION / LEARNING ===
    ref_signals = 0
    ref_kw = [
        "awesome", "awesome-list", "curated list", "list of", "resources",
        "reference", "guide", "tutorial", "how-to", "how to",
        "cookbook", "playbook", "cheat sheet", "cheatsheet", "handbook",
        "whitepaper", "white paper", "paper(s)", "bibliography",
        "reading list", "reading-list", "curriculum", "syllabus",
        "roadmap", "learning", "study", "course", "lecture",
        "workshop", "notes", "survey", "overview", "introduction",
        "getting started", "examples", "samples",
        "documentation", "docs", "doc",
    ]
    ref_topic = {
        "awesome-list", "awesome", "documentation", "tutorial",
        "educational", "learning", "reference", "paper",
        "curated-list", "resources", "guide", "how-to",
        "cheat-sheet", "roadmap", "curriculum", "notes",
    }
    for kw in ref_kw:
        if kw in text:
            ref_signals += 1
    ref_signals += len(topic_set & ref_topic) * 2
    if ref_signals >= 2:
        return "📚 Reference / Doc / List"

    # === ARCHITECTURE / DESIGN / SPECIFICATION ===
    arch_signals = 0
    arch_kw = [
        "architecture", "system design", "design pattern", "design-pattern",
        "pattern", "best practice", "best-practice", "standard",
        "specification", "spec", "protocol", "schema",
        "template", "boilerplate", "starter", "scaffold",
        "blueprint",
    ]
    arch_topic = {
        "architecture", "design", "pattern", "design-pattern",
        "best-practice", "template", "boilerplate",
    }
    for kw in arch_kw:
        if kw in text:
            arch_signals += 1
    arch_signals += len(topic_set & arch_topic) * 2
    if arch_signals >= 2:
        return "🏗️ Architecture / Design"

    # === TOOL / LIBRARY / FRAMEWORK / SDK ===
    tool_signals = 0
    tool_kw = [
        "framework", "library", "sdk", "cli", "tool", "utility",
        "plugin", "extension", "driver", "adapter", "wrapper",
        "engine", "runtime", "compiler", "interpreter",
        "database", "cache", "queue", "server", "client",
        "api", "sdk", "kit", "package",
    ]
    tool_topic = {
        "framework", "library", "sdk", "cli", "tool",
        "plugin", "driver", "database", "cache",
        "compiler", "interpreter", "runtime", "engine",
    }
    for kw in tool_kw:
        if kw in text:
            tool_signals += 1
    tool_signals += len(topic_set & tool_topic) * 2
    if tool_signals >= 2 or stars >= 1000:
        return "🔧 Tool / Lib / Framework"

    # === APPLICATION / SERVICE ===
    app_signals = 0
    app_kw = [
        "app", "application", "service", "platform", "system",
        "software", "program", "daemon", "agent", "bot",
    ]
    for kw in app_kw:
        if kw in text:
            app_signals += 1
    if app_signals >= 2 or stars >= 500:
        return "💻 App / Service"

    # Content / Design assets
    content_kw = ["font", "icon", "theme", "color", "design", "ui"]
    for kw in content_kw:
        if kw in text:
            return "🎨 Content / Design"

    # Game
    game_kw = ["game", "engine", "3d", "2d", "sprite"]
    for kw in game_kw:
        if kw in text:
            return "🎮 Game / Graphics"

    return "📦 Uncategorized"

def main():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("""
        SELECT id, name, owner, full_name, primary_language, stars,
               archive_status, quality_score, metadata_json,
               quality_json
        FROM repositories
        ORDER BY stars DESC
    """)

    all_data = []
    by_type = Counter()
    by_activity = Counter()
    by_combined = Counter()

    for row in cursor:
        (repo_id, name, owner, full_name, lang, stars,
         archived, quality, meta_json, qual_json) = row

        meta = json.loads(meta_json)
        qual = json.loads(qual_json)

        last_commit = qual.get("last_commit_date") or qual.get("last_star_update")
        activity = classify_activity(last_commit)
        by_activity[activity] += 1
        health = "healthy" if activity in ("active", "maintained") else "nonhealthy"

        license_name = meta.get("license") or "None"
        topics = meta.get("topics") or []
        description = meta.get("description") or ""
        homepage = meta.get("homepage") or ""

        repo_type = classify_repo_type(
            name, description, topics, lang or "Unknown",
            stars, homepage
        )
        by_type[repo_type] += 1
        by_combined[(health, repo_type)] += 1

        all_data.append({
            "full_name": full_name,
            "stars": stars,
            "language": lang or "Unknown",
            "health": health,
            "activity": activity,
            "archived": bool(archived),
            "repo_type": repo_type,
            "topics": topics,
            "description": description[:100],
        })

    # === OVERALL TYPE BREAKDOWN ===
    print("=" * 72)
    print("TYPE CLASSIFICATION — ALL 1245 REPOS")
    print("=" * 72)
    print()
    print(f"| {'Type':40s} | {'Count':>6s} | {'%':>6s} |")
    print(f"| {'-'*40} | {'-'*6} | {'-'*6} |")
    for t, c in sorted(by_type.items(), key=lambda x: -x[1]):
        pct = c / 1245 * 100
        print(f"| {t:40s} | {c:6d} | {pct:5.1f}% |")
    print(f"| {'TOTAL':40s} | {1245:6d} | 100% |")
    print()

    # === TYPE x HEALTH CROSS-REFERENCE ===
    print("=" * 72)
    print("TYPE x HEALTH CROSS-REFERENCE")
    print("=" * 72)
    print()
    type_order = sorted(by_type, key=lambda t: -by_type[t])
    print(f"| {'Type':40s} | {'Healthy':>8s} | {'Non-Healthy':>12s} | {'Total':>6s} |")
    print(f"| {'-'*40} | {'-'*8} | {'-'*12} | {'-'*6} |")
    for t in type_order:
        h = by_combined.get(("healthy", t), 0)
        nh = by_combined.get(("nonhealthy", t), 0)
        print(f"| {t:40s} | {h:8d} | {nh:12d} | {h+nh:6d} |")
    print()

    # === HEALTHY REFS THAT ARE IN THE WRONG PLACE ===
    print("=" * 72)
    print("🔍 HEALTHY REPOS THAT ARE CLEARLY REFERENCE/DOC/LIST TYPE")
    print("=" * 72)
    print()
    healthy_refs = [r for r in all_data
                    if r["health"] == "healthy"
                    and r["repo_type"].startswith("📚")]
    print(f"Found {len(healthy_refs)} healthy repos that are Reference/Doc/List type")
    print()
    for r in healthy_refs:
        print(f"  {r['full_name']:50s} ⭐{r['stars']:>7d} {r['language']:15s} {r['description'][:60]}")
    print()

    # === NON-HEALTHY REFS THAT SHOULD STAY ===
    print("=" * 72)
    print("🔍 NON-HEALTHY REFERENCE/DOC/ARCHITECTURE REPS (the keepers)")
    print("=" * 72)
    print()
    nonhealthy_keepers = [r for r in all_data
                          if r["health"] == "nonhealthy"
                          and (r["repo_type"].startswith("📚")
                               or r["repo_type"].startswith("🏗️"))]
    print(f"Found {len(nonhealthy_keepers)} non-healthy Reference/Architecture repos (keepers)")
    print()
    for r in nonhealthy_keepers[:30]:
        print(f"  {r['full_name']:50s} ⭐{r['stars']:>7d} {r['activity']:>10s} {r['description'][:60]}")
    if len(nonhealthy_keepers) > 30:
        print(f"  ... and {len(nonhealthy_keepers) - 30} more")
    print()

    # === NON-HEALTHY TOOLS (also keepers) ===
    print("=" * 72)
    print("🔍 NON-HEALTHY TOOL/LIB/FRAMEWORK REPS (also keepers)")
    print("=" * 72)
    print()
    nonhealthy_tools = [r for r in all_data
                        if r["health"] == "nonhealthy"
                        and r["repo_type"].startswith("🔧")]
    print(f"Found {len(nonhealthy_tools)} non-healthy Tool repos (potential keepers)")
    for r in nonhealthy_tools[:20]:
        print(f"  {r['full_name']:50s} ⭐{r['stars']:>7d} {r['activity']:>10s} {r['description'][:60]}")
    if len(nonhealthy_tools) > 20:
        print(f"  ... and {len(nonhealthy_tools) - 20} more")
    print()

    # === ACTIVITY BREAKDOWN PER TYPE ===
    print("=" * 72)
    print("ACTIVITY BREAKDOWN PER TYPE")
    print("=" * 72)
    print()
    type_activity = defaultdict(Counter)
    for r in all_data:
        type_activity[r["repo_type"]][r["activity"]] += 1
    for t in type_order:
        c = type_activity[t]
        total = sum(c.values())
        parts = ", ".join(f"{k}: {c[k]}" for k in ["active", "maintained", "stale", "abandoned", "unknown"] if c[k] > 0)
        print(f"  {t:40s} total={total:4d}  [{parts}]")
    print()

    # === SUMMARY ===
    print("=" * 72)
    print("RECOMMENDED ACTIONS")
    print("=" * 72)
    print()
    print(f"Health breakdown: {by_activity['active']:4d} active, {by_activity['maintained']:4d} maintained, ", end="")
    print(f"{by_activity['stale']:4d} stale, {by_activity['abandoned']:4d} abandoned")
    print()
    print(f"Healthy Reference/Doc repos:       {len(healthy_refs):4d} — tag with ref:docs")
    print(f"Non-healthy Reference/Doc keepers: {len(nonhealthy_keepers):4d} — tag with ref:archived-ref")
    print(f"Non-healthy Tool keepers:          {len(nonhealthy_tools):4d} — tag with ref:archived-tool")
    print(f"Total potential keepers:           {len(healthy_refs) + len(nonhealthy_keepers) + len(nonhealthy_tools):4d}")
    print()

    conn.close()

if __name__ == "__main__":
    main()
