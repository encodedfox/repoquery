#!/usr/bin/env python3
"""Analyze active/maintained repos for domain categorization."""
import json
import sqlite3
from collections import Counter, defaultdict
from datetime import datetime, timedelta

DB_PATH = "data/repoquery.db"
NOW = datetime.now()

# Activity thresholds (same as Rust code)
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

# Star ranges for adoption analysis
STAR_RANGES = [
    (0, 10, "0-10 (Minimal)"),
    (10, 100, "10-100 (Low)"),
    (100, 1000, "100-1k (Moderate)"),
    (1000, 10000, "1k-10k (High)"),
    (10000, float('inf'), "10k+ (Very High)"),
]

# Quality score ranges
QUALITY_RANGES = [
    (0, 20, "0-20 (Poor)"),
    (20, 40, "20-40 (Below Avg)"),
    (40, 60, "40-60 (Average)"),
    (60, 80, "60-80 (Good)"),
    (80, 101, "80-100 (Excellent)"),
]

# Domain keyword classification
DOMAIN_KEYWORDS = {
    "AI/ML/Data": ["machine learning", "deep learning", "artificial intelligent", "llm", "gpt", "ai ", " ml", "neural", "tensorflow", "pytorch", "data science", "data pipeline", "data engineering", "analytics", "data processing", "data visualization", "big data", "data lake", "data warehouse", "etl"],
    "DevOps/Infrastructure": ["kubernetes", "k8s", "docker", "container", "devops", "ci/cd", "continuous integration", "continuous deployment", "infrastructure", "terraform", "ansible", "helm", "monitoring", "observability", "sre", "platform engineering", "cloud native", "deployment"],
    "Databases/Storage": ["database", "sql", "nosql", "storage", "object store", "caching", "cache", "distributed database", "sqlite", "postgresql", "mysql", "redis", "s3", "key-value", "time-series"],
    "Security/Compliance": ["security", "compliance", "vulnerability", "audit", "encryption", "authentication", "authorization", "iam", "secret", "zero trust", "penetration", "cve", "sbom", "supply chain"],
    "Web/Frontend": ["web", "frontend", "react", "vue", "angular", "css", "html", "javascript", "typescript", "ui", "ux", "component", "design system", "documentation"],
    "CLI/Developer Tools": ["cli", "command line", "terminal", "developer tools", "debugger", "profiler", "linter", "formatter", "code analysis", "static analysis", "code review", "git"],
    "Networking/Communication": ["network", "http", "grpc", "rest", "api", "message queue", "pub/sub", "event", "streaming", "websocket", "protocol", "proxy", "load balancer"],
    "Programming Languages/Runtimes": ["compiler", "interpreter", "programming language", "runtime", "vm", "virtual machine", "language server", "package manager"],
    "Blockchain/Distributed Systems": ["blockchain", "distributed system", "consensus", "hyperledger", "ethereum", "crypto", "smart contract", "dlt"],
    "Automation/Workflow": ["automation", "workflow", "orchestration", "scheduler", "task", "pipeline", "ci", "bot", "chatbot"],
    "Scientific/Engineering": ["scientific", "engineering", "simulation", "physics", "biology", "chemistry", "math", "optimization", "computational"],
    "Media/Content": ["media", "video", "audio", "image", "font", "icon", "design", "creative", "generative art", "3d", "animation"],
}

def classify_domain(name, description, topics, language):
    text = f"{name} {description} {language}".lower()
    text += " " + " ".join(t.lower() for t in (topics or []))
    scores = {}
    for domain, keywords in DOMAIN_KEYWORDS.items():
        score = sum(1 for kw in keywords if kw.lower() in text)
        if score > 0:
            scores[domain] = score
    if scores:
        return max(scores, key=scores.get)
    # Fallback based on language
    lang_domain = {
        "Go": "DevOps/Infrastructure",
        "Rust": "CLI/Developer Tools",
        "Python": "AI/ML/Data",
        "JavaScript": "Web/Frontend",
        "TypeScript": "Web/Frontend",
        "Java": "Web/Frontend",
        "Ruby": "Web/Frontend",
        "PHP": "Web/Frontend",
        "C": "Systems/Embedded",
        "C++": "Systems/Embedded",
        "Shell": "DevOps/Infrastructure",
        "Dockerfile": "DevOps/Infrastructure",
        "HCL": "DevOps/Infrastructure",
    }
    return lang_domain.get(language, "Other")

def main():
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("""
        SELECT id, name, owner, full_name, primary_language, stars,
               archive_status, quality_score, metadata_json,
               quality_json
        FROM repositories
    """)

    healthy = []
    all_classified = Counter()

    for row in cursor:
        (repo_id, name, owner, full_name, lang, stars,
         archived, quality, meta_json, qual_json) = row

        meta = json.loads(meta_json)
        qual = json.loads(qual_json)

        last_commit = qual.get("last_commit_date") or qual.get("last_star_update")
        activity = classify_activity(last_commit)
        all_classified[activity] += 1

        if activity in ("active", "maintained"):
            license_name = meta.get("license") or "None"
            license_spdx = meta.get("license_spdx") or ""
            topics = meta.get("topics") or []
            description = meta.get("description") or ""
            homepage = meta.get("homepage") or ""
            language_breakdown = meta.get("language_breakdown")

            domain = classify_domain(name, description, topics, lang or "Unknown")

            healthy.append({
                "id": repo_id,
                "name": name,
                "owner": owner,
                "full_name": full_name,
                "language": lang or "Unknown",
                "stars": stars,
                "quality": quality,
                "activity": activity,
                "license": license_name,
                "license_spdx": license_spdx,
                "topics": topics,
                "description": description,
                "homepage": homepage,
                "domain": domain,
                "last_commit": last_commit or "unknown",
                "language_breakdown": language_breakdown,
                "fork_ahead": None,
                "fork_behind": None,
            })

    print(f"Total classified: {sum(all_classified.values())}")
    print(f"  Active: {all_classified['active']}")
    print(f"  Maintained: {all_classified['maintained']}")
    print(f"  Stale: {all_classified['stale']}")
    print(f"  Abandoned: {all_classified['abandoned']}")
    print(f"  Unknown: {all_classified['unknown']}")
    print(f"  Healthy (active + maintained): {len(healthy)}")
    print()

    # === LICENSE ANALYSIS ===
    print("=" * 72)
    print("LICENSE DISTRIBUTION")
    print("=" * 72)
    license_counts = Counter(r["license"] for r in healthy)
    for lic, count in license_counts.most_common(20):
        pct = count / len(healthy) * 100
        print(f"  {lic:45s} {count:4d} ({pct:5.1f}%)")
    print(f"  {'TOTAL':45s} {len(healthy):4d} (100%)")
    print()

    # === STAR ADOPTION ===
    print("=" * 72)
    print("ADOPTION (STAR RANGES)")
    print("=" * 72)
    star_dist = Counter()
    for r in healthy:
        for lo, hi, label in STAR_RANGES:
            if lo <= r["stars"] < hi:
                star_dist[label] += 1
                break
    def star_sort_key(item):
        label = item[0]
        raw = label.split("-")[0].strip()
        if raw.endswith("k"):
            return int(float(raw[:-1]) * 1000)
        if raw.endswith("+"):
            raw = raw[:-1]
            if raw.endswith("k"):
                return int(float(raw[:-1]) * 1000)
        return int(raw) if raw.isdigit() else 0
    for label, count in sorted(star_dist.items(), key=star_sort_key):
        pct = count / len(healthy) * 100
        print(f"  {label:30s} {count:4d} ({pct:5.1f}%)")
    print(f"  {'TOTAL':30s} {len(healthy):4d} (100%)")
    print()

    # === QUALITY SCORES ===
    print("=" * 72)
    print("QUALITY SCORE DISTRIBUTION")
    print("=" * 72)
    qual_dist = Counter()
    for r in healthy:
        for lo, hi, label in QUALITY_RANGES:
            if lo <= r["quality"] < hi:
                qual_dist[label] += 1
                break
    def qual_sort_key(item):
        return int(item[0].split("-")[0].strip())
    for label, count in sorted(qual_dist.items(), key=qual_sort_key):
        pct = count / len(healthy) * 100
        print(f"  {label:30s} {count:4d} ({pct:5.1f}%)")
    print(f"  {'TOTAL':30s} {len(healthy):4d} (100%)")
    print()

    # === LANGUAGE BREAKDOWN ===
    print("=" * 72)
    print("LANGUAGE BREAKDOWN")
    print("=" * 72)
    lang_counts = Counter(r["language"] for r in healthy)
    for lang, count in lang_counts.most_common(20):
        pct = count / len(healthy) * 100
        print(f"  {lang:30s} {count:4d} ({pct:5.1f}%)")
    print(f"  {'TOTAL':30s} {len(healthy):4d} (100%)")
    print()

    # === TOPIC ANALYSIS (top topics across healthy repos) ===
    print("=" * 72)
    print("TOP TOPICS (ACROSS HEALTHY REPOS)")
    print("=" * 72)
    topic_counts = Counter()
    for r in healthy:
        for t in r["topics"]:
            topic_counts[t] += 1
    for topic, count in topic_counts.most_common(30):
        pct = count / len(healthy) * 100
        print(f"  {topic:45s} {count:4d} ({pct:4.1f}%)")
    print()

    # === DOMAIN CATEGORIZATION ===
    print("=" * 72)
    print("DOMAIN CATEGORIZATION")
    print("=" * 72)
    domain_counts = Counter(r["domain"] for r in healthy)
    for domain, count in domain_counts.most_common():
        pct = count / len(healthy) * 100
        print(f"  {domain:40s} {count:4d} ({pct:5.1f}%)")
    print()

    # === CROSS-REFERENCE: DOMAIN x LANGUAGE ===
    print("=" * 72)
    print("CROSS-REFERENCE: DOMAIN x TOP LANGUAGES")
    print("=" * 72)
    domain_lang = defaultdict(Counter)
    for r in healthy:
        domain_lang[r["domain"]][r["language"]] += 1
    for domain in sorted(domain_lang.keys()):
        top_langs = domain_lang[domain].most_common(5)
        lang_str = ", ".join(f"{l} ({c})" for l, c in top_langs)
        print(f"  {domain:30s} → {lang_str}")
    print()

    # === CROSS-REFERENCE: DOMAIN x STAR RANGES ===
    print("=" * 72)
    print("CROSS-REFERENCE: DOMAIN x ADOPTION")
    print("=" * 72)
    domain_stars = defaultdict(Counter)
    for r in healthy:
        for lo, hi, label in STAR_RANGES:
            if lo <= r["stars"] < hi:
                domain_stars[r["domain"]][label] += 1
                break
    for domain in sorted(domain_stars.keys()):
        total_d = sum(domain_stars[domain].values())
        star_strs = [f"{l}:{c}" for l, c in domain_stars[domain].most_common(3)]
        print(f"  {domain:30s} (total: {total_d:4d}) {', '.join(star_strs)}")
    print()

    # === TOP REPOS BY DOMAIN (top 5 per domain by stars) ===
    print("=" * 72)
    print("TOP REPOS PER DOMAIN (BY STARS)")
    print("=" * 72)
    domain_repos = defaultdict(list)
    for r in healthy:
        domain_repos[r["domain"]].append(r)
    for domain in sorted(domain_repos.keys()):
        print(f"\n  [{domain}]")
        top = sorted(domain_repos[domain], key=lambda x: -x["stars"])[:5]
        for r in top:
            desc_short = r["description"][:70] if r["description"] else ""
            print(f"    {r['full_name']:45s} ⭐{r['stars']:>6d}  {r['license'][:20]:20s}  {desc_short}")

    # === LICENSE x DOMAIN CROSS-REFERENCE ===
    print()
    print("=" * 72)
    print("LICENSE x DOMAIN CROSS-REFERENCE")
    print("=" * 72)
    license_domain = defaultdict(lambda: defaultdict(int))
    for r in healthy:
        lic = r["license"]
        if lic == "Other" or not lic:
            lic = "Other/None"
        license_domain[r["domain"]][lic] += 1
    for domain in sorted(license_domain.keys()):
        top_lics = sorted(license_domain[domain].items(), key=lambda x: -x[1])[:3]
        lic_str = ", ".join(f"{l} ({c})" for l, c in top_lics)
        print(f"  {domain:30s} → {lic_str}")

    conn.close()

if __name__ == "__main__":
    main()
