#!/usr/bin/env python3
"""Analyze stale/abandoned/archived repos for reference/documentation value."""
import json
import sqlite3
from collections import Counter, defaultdict
from datetime import datetime, timedelta

DB_PATH = "data/repoquery.db"
NOW = datetime.now()

ACTIVE_MONTHS = 3
STALE_MONTHS = 12

# Keywords that suggest reference/documentation/architecture value
REFERENCE_KEYWORDS = {
    "documentation": "📚",
    "doc": "📚",
    "docs": "📚",
    "awesome": "📚",
    "awesome-list": "📚",
    "curated list": "📚",
    "list of": "📚",
    "resources": "📚",
    "reference": "📚",
    "guide": "📚",
    "tutorial": "📚",
    "how-to": "📚",
    "how to": "📚",
    "cookbook": "📚",
    "playbook": "📚",
    "cheat sheet": "📚",
    "handbook": "📚",
    "whitepaper": "📚",
    "white paper": "📚",
    "paper": "📚",
    "bibliography": "📚",
    "reading list": "📚",
    "reading-list": "📚",
    "curriculum": "📚",
    "syllabus": "📚",
    "roadmap": "📚",
    "learning": "📚",
    "study": "📚",
    "course": "📚",
    "lecture": "📚",
    "workshop": "📚",
    "notes": "📚",
    "cheatsheet": "📚",
    "survey": "📚",
    "overview": "📚",
    "introduction": "📚",
    "getting started": "📚",
    "examples": "📚",
    "samples": "📚",
    "blueprint": "📚",
    "architecture": "🏗️",
    "system design": "🏗️",
    "design pattern": "🏗️",
    "design-pattern": "🏗️",
    "pattern": "🏗️",
    "best practice": "🏗️",
    "best-practice": "🏗️",
    "standard": "🏗️",
    "specification": "🏗️",
    "spec": "🏗️",
    "protocol": "🏗️",
    "schema": "🏗️",
    "template": "🏗️",
    "boilerplate": "🏗️",
    "starter": "🏗️",
    "scaffold": "🏗️",
    "framework": "🔧",
    "library": "🔧",
    "sdk": "🔧",
    "api": "🔧",
    "cli": "🔧",
    "tool": "🔧",
    "utility": "🔧",
    "plugin": "🔧",
    "extension": "🔧",
    "driver": "🔧",
    "adapter": "🔧",
    "wrapper": "🔧",
}

# Topic-based categorization
REFERENCE_TOPICS = {
    "awesome-list", "awesome", "documentation", "tutorial", "educational",
    "learning", "reference", "paper", "specification", "standard",
    "curated-list", "resources", "guide", "how-to", "cookbook",
    "cheat-sheet", "roadmap", "curriculum", "course", "notes",
    "architecture", "design", "pattern", "design-pattern", "best-practice",
    "template", "boilerplate", "starter", "example", "demo",
    "sample", "blueprint", "handbook", "playbook",
}

# Topics that mark something as a truly useful project despite being stale
VALUE_TOPICS = {
    "database", "cache", "queue", "monitoring", "logging",
    "security", "networking", "compression", "serialization",
    "testing", "ci", "deployment", "container", "orchestration",
    "search", "indexing", "analytics",
}

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

def classify_keep_type(name, description, topics, language, stars, homepage, is_archived):
    text = f"{name} {description}".lower()
    topic_set = {t.lower() for t in (topics or [])}
    all_text = text + " " + " ".join(topic_set)

    # Detect reference/documentation type
    has_doc_ref = any(kw in all_text for kw in [
        "awesome", "list", "reference", "guide", "tutorial",
        "cookbook", "cheat sheet", "handbook", "roadmap",
        "curriculum", "course", "learning", "study", "paper",
        "documentation", "reading list", "notes", "resources",
        "how to", "getting started",
    ])
    has_arch_ref = any(kw in all_text for kw in [
        "architecture", "system design", "design pattern",
        "pattern", "best practice", "standard", "specification",
        "spec", "protocol", "template", "blueprint",
    ])
    has_tool_value = (
        stars >= 1000
        or bool(topic_set & VALUE_TOPICS)
        or any(kw in all_text for kw in [
            "framework", "library", "database", "engine",
            "runtime", "compiler", "language",
        ])
    )

    # Extra points for well-known projects with high stars
    if stars >= 10000:
        has_reference_value = True
        return "📚 Reference", "high-value-reference"
    elif stars >= 5000 and has_doc_ref:
        return "📚 Reference", "popular-reference"
    elif stars >= 5000 and (has_arch_ref or has_tool_value):
        return "🏗️ Architecture" if has_arch_ref else "🔧 Tool", "popular-project"
    elif stars >= 1000 and has_doc_ref:
        return "📚 Reference", "reference"
    elif has_doc_ref and not has_tool_value:
        return "📚 Reference", "documentation"
    elif has_arch_ref:
        return "🏗️ Architecture", "architecture"
    elif has_tool_value:
        return "🔧 Tool", "useful-tool"
    elif is_archived:
        return "🗑️ Archived", "archived-no-reference"
    else:
        return "🗑️ Cleanup", "stale-no-value"

def main():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("""
        SELECT id, name, owner, full_name, primary_language, stars,
               archive_status, quality_score, metadata_json,
               quality_json
        FROM repositories
    """)

    by_type = defaultdict(list)
    classified_total = Counter()
    non_healthy = []

    for row in cursor:
        (repo_id, name, owner, full_name, lang, stars,
         archived, quality, meta_json, qual_json) = row

        meta = json.loads(meta_json)
        qual = json.loads(qual_json)

        last_commit = qual.get("last_commit_date") or qual.get("last_star_update")
        activity = classify_activity(last_commit)
        classified_total[activity] += 1

        if activity not in ("active", "maintained"):
            license_name = meta.get("license") or "None"
            topics = meta.get("topics") or []
            description = meta.get("description") or ""
            homepage = meta.get("homepage") or ""

            keep_type, keep_reason = classify_keep_type(
                name, description, topics, lang or "Unknown",
                stars, homepage, bool(archived)
            )

            non_healthy.append({
                "id": repo_id,
                "full_name": full_name,
                "language": lang or "Unknown",
                "stars": stars,
                "activity": activity,
                "archived": bool(archived),
                "license": license_name,
                "topics": topics,
                "description": description[:120] if description else "",
                "homepage": homepage,
                "last_commit": last_commit or "unknown",
                "quality": quality,
                "keep_type": keep_type,
                "keep_reason": keep_reason,
                "name": name,
            })
            by_type[keep_type].append(non_healthy[-1])

    # === HEADER ===
    print("# Non-Healthy Repos: Keep vs Cleanup Analysis")
    print()
    print(f"Generated: {NOW.strftime('%Y-%m-%d %H:%M')}")
    print()

    # === OVERVIEW ===
    print("## Classification Counts")
    print()
    print(f"| Activity | Count |")
    print(f"|----------|-------|")
    for k in ["active", "maintained", "stale", "abandoned", "unknown"]:
        print(f"| {k.capitalize():10s} | {classified_total[k]:5d} |")
    print(f"| {'Non-healthy total':10s} | {len(non_healthy):5d} |")
    print()

    # === KEEP TYPE BREAKDOWN ===
    print("## Keep-Type Breakdown")
    print()
    print(f"| Keep Type | Count |")
    print(f"|-----------|-------|")
    type_order = ["📚 Reference", "🏗️ Architecture", "🔧 Tool", "🗑️ Archived", "🗑️ Cleanup"]
    for t in type_order:
        if t in by_type:
            print(f"| {t:20s} | {len(by_type[t]):5d} |")
    print()

    # === SECTION: KEEPERS (Reference / Architecture / Tool) ===
    keep_types = {
        "📚 Reference": "Reference / Documentation — likely worth keeping for learning/reference",
        "🏗️ Architecture": "Architecture / Design — patterns, templates, specs, standards",
        "🔧 Tool": "Tool / Library — useful despite being unmaintained",
    }

    print("## 🟢 KEEP: Repos Worth Keeping for Reference")
    print()

    for kt, desc in keep_types.items():
        repos = sorted(by_type.get(kt, []), key=lambda r: -r["stars"])
        if not repos:
            continue
        print(f"### {kt}: {desc}")
        print()
        print(f"_{len(repos)} repos_")
        print()
        print(f"| {'Repo':50s} | {'Lang':15s} | {'Stars':>8s} | {'Activity':12s} | {'License':25s} | {'Description / Signal'} |")
        print(f"| {'-'*50} | {'-'*15} | {'-'*8} | {'-'*12} | {'-'*25} | {'-'*60} |")
        for r in repos[:50]:  # Top 50 per type
            desc_short = r["description"][:60] if r["description"] else ""
            activity_label = f"{'📦 archived' if r['archived'] else r['activity']}"
            print(f"| {r['full_name']:50s} | {r['language']:15s} | {r['stars']:>8d} | {activity_label:12s} | {r['license'][:25]:25s} | {desc_short}")
        if len(repos) > 50:
            print(f"| ... and {len(repos) - 50} more |")
        print()

    # === SECTION: CLEANUP CANDIDATES ===
    cleanup_types = {
        "🗑️ Archived": "Archived — no clear reference value (consider unstarring)",
        "🗑️ Cleanup": "Stale/Abandoned — no clear reference value (consider unstarring)",
    }

    print("## 🔴 REVIEW: Repos to Consider Unstarring")
    print()

    for kt, desc in cleanup_types.items():
        repos = sorted(by_type.get(kt, []), key=lambda r: -r["stars"])
        if not repos:
            continue
        print(f"### {kt}: {desc}")
        print()
        print(f"_{len(repos)} repos_")
        print()
        for r in repos[:30]:  # Top 30 per type
            desc_short = r["description"][:80] if r["description"] else ""
            print(f"  • {r['full_name']:45s} ⭐{r['stars']:>6d} {r['activity']:>10s} {desc_short}")
        if len(repos) > 30:
            print(f"  ... and {len(repos) - 30} more")
        print()

    # === SUGGESTED TAGS ===
    print("## Suggested Tags for Keepers")
    print()
    print("After review, apply tags with:")
    print()
    print("```")
    for kt in ["📚 Reference", "🏗️ Architecture", "🔧 Tool"]:
        for r in sorted(by_type.get(kt, []), key=lambda x: -x["stars"])[:50]:
            tag = f"ref:{r['keep_reason']}"
            print(f"repoquery repo tag {r['full_name']} {tag}")
    print("```")
    print()

    # === STATISTICS ===
    print("## Appendix: Signals Used")
    print()
    print("| Signal | Weight |")
    print("|--------|--------|")
    print("| Description contains 'awesome', 'list', 'reference', 'guide', 'tutorial', etc. | 📚 Reference |")
    print("| Description contains 'architecture', 'design pattern', 'specification', 'template', etc. | 🏗️ Architecture |")
    print("| Stars ≥ 5,000 | Reference / High-value (regardless of description) |")
    print("| Stars ≥ 10,000 | High-value reference (always kept) |")
    print("| Topics: database, cache, queue, monitoring, security, etc. | 🔧 Tool |")
    print("| Archived + no reference signals | 🗑️ Archived / Review |")
    print("| Stale/Abandoned + no reference signals | 🗑️ Cleanup / Review |")
    print()

    conn.close()

if __name__ == "__main__":
    main()
