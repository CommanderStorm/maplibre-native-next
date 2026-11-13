#!/usr/bin/env python3
"""Files or updates the nightly benchmark issue (README.md, Nightly):

    nightly.py --report <report.md> --base <sha> --head <sha> [--dry-run]

Reads `mln-bench compare`'s report of main (head) against main a day
earlier (base). When a case got a worse verdict, it opens the issue titled
TITLE with the report, or comments on it when it is open already. When none
did, it closes an open one with a comment saying so. Through the gh CLI,
which the workflow logs in with its token; --dry-run prints what it would do.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys

TITLE = "Nightly benchmarks: main got worse"


def worse_rows(report: str) -> list[list[str]]:
    """The report's table rows with a worse verdict, as their cells."""
    rows = []
    for line in report.splitlines():
        if line.startswith("|") and "**worse**" in line:
            rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return rows


def open_issue(repo: str | None) -> int | None:
    """The number of the open issue titled TITLE, if there is one."""
    args = ["gh", "issue", "list", "--state", "open", "--search", f'in:title "{TITLE}"', "--json", "number,title"]
    if repo:
        args += ["--repo", repo]
    issues = json.loads(subprocess.run(args, check=True, capture_output=True, text=True).stdout)
    return next((issue["number"] for issue in issues if issue["title"] == TITLE), None)


def verdicts(count: int) -> str:
    return f"{count} worse verdict" + ("" if count == 1 else "s")


def body(report: str, rows: list[list[str]], base: str, head: str) -> str:
    lines = [
        f"Main at {head} against main a day earlier, {base}, gave {verdicts(len(rows))}. "
        "The runner draws on lavapipe, where the times are noisy; allocations count the "
        "same as on any machine.",
        "",
        "| case | metric | base | candidate | change | p |",
        "|---|---|--:|--:|--:|--:|",
    ]
    lines += ["| " + " | ".join(row[:6]) + " |" for row in rows]
    lines += ["", "<details><summary>The full report</summary>", "", report.strip(), "", "</details>"]
    return "\n".join(lines) + "\n"


def gh(args: list[str], repo: str | None, dry_run: bool, text: str | None = None) -> None:
    args = ["gh", *args] + (["--repo", repo] if repo else [])
    if dry_run:
        print("would run:", " ".join(args) + (f" <<{len(text)} characters>>" if text else ""))
        return
    subprocess.run(args, check=True, input=text, text=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", required=True)
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--repo")
    parser.add_argument("--dry-run", action="store_true")
    options = parser.parse_args()

    with open(options.report, encoding="utf-8") as file:
        report = file.read()
    if "better, " not in report:
        print("nightly: the report has no verdicts line; the comparison didn't finish", file=sys.stderr)
        return 1
    rows = worse_rows(report)
    issue = open_issue(options.repo)
    if rows:
        text = body(report, rows, options.base, options.head)
        if issue is None:
            gh(["issue", "create", "--title", TITLE, "--body-file", "-"], options.repo, options.dry_run, text)
        else:
            gh(["issue", "comment", str(issue), "--body-file", "-"], options.repo, options.dry_run, text)
        print(f"nightly: {verdicts(len(rows))}")
    elif issue is not None:
        text = f"Main at {options.head} against {options.base} gave no worse verdicts.\n"
        gh(["issue", "comment", str(issue), "--body-file", "-"], options.repo, options.dry_run, text)
        gh(["issue", "close", str(issue)], options.repo, options.dry_run)
        print("nightly: no worse verdicts, closed the issue")
    else:
        print("nightly: no worse verdicts")
    return 0


if __name__ == "__main__":
    sys.exit(main())
