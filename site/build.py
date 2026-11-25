#!/usr/bin/env python3
"""Renders the README as the website: site/index.html.

Run it after the README changes:  python3 site/build.py
It uses only the standard library.
"""

import html
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
README = ROOT.parent / "README.md"

KEYWORDS = set(
    "let mortal law parable give amen if otherwise while repeat times for each in break continue "
    "tempt forgive sin may preach deny and or is not lend entrust discern kind pagan baptize as".split()
)
TYPES = set("Number Decimal Text Truth Maybe Pagan Nothing Found Lost".split())
BUILTINS = set(
    "length push pop get lookup contains remove keys copy range sort reverse split join trim upper lower "
    "replace chars text number decimal floor round abs min max read_file write_file read_line args "
    "parse_json to_json".split()
)

TOKEN = re.compile(
    r"""(?P<comment>--[^\n]*)
      | (?P<string>"(?:\\.|[^"\\\n])*")
      | (?P<number>\b\d[\d_]*(?:\.\d+)?\b)
      | (?P<word>[A-Za-zÀ-ÿ_][A-Za-zÀ-ÿ0-9_]*)
      | (?P<other>.|\n)""",
    re.X,
)


def span(cls, text):
    return f'<span class="{cls}">{html.escape(text)}</span>'


def highlight_string(text):
    """A text, with the `{values}` placed inside it marked."""
    out, i = [], 0
    for m in re.finditer(r"\\.|\{[^{}\n]*\}", text):
        out.append(span("s", text[i : m.start()]))
        piece = m.group()
        out.append(span("e", piece) if piece.startswith("\\") else span("i", piece))
        i = m.end()
    out.append(span("s", text[i:]))
    return "".join(out)


def highlight_gesu(code):
    out = []
    for m in TOKEN.finditer(code):
        kind, text = m.lastgroup, m.group()
        if kind == "comment":
            out.append(span("c", text))
        elif kind == "string":
            out.append(highlight_string(text))
        elif kind == "number":
            out.append(span("n", text))
        elif kind == "word":
            if text == "Jesus":
                out.append(span("j", text))
            elif text == "False":
                out.append(span("f", text))
            elif text in ("True", "true"):
                out.append(span("h", text))
            elif text in KEYWORDS:
                out.append(span("k", text))
            elif text in TYPES or text[0].isupper():
                out.append(span("t", text))
            elif text in BUILTINS:
                out.append(span("b", text))
            else:
                out.append(html.escape(text))
        else:
            out.append(html.escape(text))
    return "".join(out)


def highlight_report(text):
    """What the compiler says: the label in red, the notes dimmed."""
    lines = []
    for line in text.split("\n"):
        m = re.match(r"^(error|warning|sin|mortal sin)(: )(.*)$", line)
        if m:
            cls = "w" if m.group(1) == "warning" else "x"
            lines.append(span(cls, m.group(1)) + html.escape(m.group(2)) + span("m", m.group(3)))
        elif re.match(r"^\s*(-->|=|\d*\s*\|)", line):
            lines.append(span("d", line))
        else:
            lines.append(html.escape(line))
    return "\n".join(lines)


def code_block(lang, text):
    is_report = re.search(r"^(error|warning|sin|mortal sin): ", text, re.M)
    # GitHub cannot color Gesù, so the README labels Gesù code as Lua, the
    # nearest language it knows. Here it gets its real colors back.
    is_gesu = lang in ("", "lua") and re.search(r"^\s*(let|law|parable|preach|kind|pagan|discern|tempt)\b", text, re.M)
    if lang and not is_gesu:
        return f'<pre class="shell"><code>{html.escape(text)}</code></pre>'
    if is_report:
        return f'<pre class="report"><code>{highlight_report(text)}</code></pre>'
    if is_gesu:
        return f'<pre class="gesu"><code>{highlight_gesu(text)}</code></pre>'
    return f'<pre class="output"><code>{html.escape(text)}</code></pre>'


def inline(text):
    """Bold, code and links inside a line of prose."""
    parts = re.split(r"(`[^`]+`)", text)
    out = []
    for part in parts:
        if part.startswith("`") and part.endswith("`") and len(part) > 1:
            out.append(f"<code>{html.escape(part[1:-1])}</code>")
            continue
        part = html.escape(part, quote=False)
        part = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", part)
        part = re.sub(r"\*(.+?)\*", r"<em>\1</em>", part)
        part = re.sub(r"\[([^\]]+)\]\((https?://[^)\s]+)\)", r'<a href="\2">\1</a>', part)
        part = re.sub(r"(?<![\">])(https?://[^\s<]+[^\s<.,)])", r'<a href="\1">\1</a>', part)
        out.append(part)
    return "".join(out)


def slug(text):
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def table(rows):
    cells = [[c.strip() for c in row.strip().strip("|").split("|")] for row in rows]
    head, body = cells[0], cells[2:]
    out = ["<div class=\"table\"><table><thead><tr>"]
    out += [f"<th>{inline(c)}</th>" for c in head]
    out.append("</tr></thead><tbody>")
    for row in body:
        out.append("<tr>" + "".join(f"<td>{inline(c)}</td>" for c in row) + "</tr>")
    out.append("</tbody></table></div>")
    return "".join(out)


# Shorter names for the side contents, where the heading itself is long.
SHORT = {"What makes Gesù the most examined language you can install today?": "The 14 facts"}


def render(markdown):
    """Returns the page body and the list of its sections as (id, label)."""
    lines = markdown.split("\n")
    out, i = [], 0
    sections = []
    while i < len(lines):
        line = lines[i]
        if line.startswith("```"):
            lang = line[3:].strip()
            i += 1
            block = []
            while i < len(lines) and not lines[i].startswith("```"):
                block.append(lines[i])
                i += 1
            out.append(code_block(lang, "\n".join(block)))
            i += 1
        elif m := re.match(r"^(#{1,3}) (.*)$", line):
            level, title = len(m.group(1)), m.group(2)
            if level == 2 and sections is not None and out and not out[-1].startswith("<h1"):
                label = SHORT.get(title) or re.split(r"[:,]", title)[0]
                sections.append((slug(title), label))
            out.append(f'<h{level} id="{slug(title)}">{inline(title)}</h{level}>')
            i += 1
        elif line.startswith("|"):
            rows = []
            while i < len(lines) and lines[i].startswith("|"):
                rows.append(lines[i])
                i += 1
            out.append(table(rows))
        elif line.startswith("- "):
            out.append("<ul>")
            while i < len(lines) and lines[i].startswith("- "):
                out.append(f"<li>{inline(lines[i][2:])}</li>")
                i += 1
            out.append("</ul>")
        elif re.match(r"^\d+\. ", line):
            out.append("<ol>")
            while i < len(lines) and re.match(r"^\d+\. ", lines[i]):
                out.append(f"<li>{inline(re.sub(r'^\d+\. ', '', lines[i]))}</li>")
                i += 1
            out.append("</ol>")
        elif not line.strip():
            i += 1
        else:
            para = []
            while i < len(lines) and lines[i].strip() and not re.match(r"^(```|#{1,3} |\||- )", lines[i]):
                para.append(lines[i])
                i += 1
            text = " ".join(para)
            # The numbered facts get a numeral of their own.
            if m := re.match(r"^\*\*(\d+)\.\*\* (.*)$", text):
                out.append(f'<div class="fact"><span class="numeral">{m.group(1)}</span><p>{inline(m.group(2))}</p></div>')
            else:
                out.append(f"<p>{inline(text)}</p>")
    return "\n".join(out), sections


PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Gesù: there is only one Truth, Jesus</title>
<meta name="description" content="Gesù is a programming language with two booleans. One is False. The other is Jesus.">
<meta property="og:title" content="Gesù: there is only one Truth, Jesus">
<meta property="og:description" content="A programming language with two booleans. One is False. The other is Jesus.">
<meta property="og:type" content="website">
<link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Crect width='32' height='32' rx='6' fill='%238f1d1d'/%3E%3Cpath d='M14 5h4v7h7v4h-7v11h-4V16H7v-4h7z' fill='%23f6efe0'/%3E%3C/svg%3E">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Cormorant+Garamond:ital,wght@0,500;0,600;0,700;1,500&family=EB+Garamond:ital,wght@0,400;0,500;0,600;1,400&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
<style>
:root {
  --parchment: #f6efe0;
  --page: #fbf7ec;
  --ink: #221b16;
  --soft: #6b5d51;
  --rubric: #8f1d1d;
  --gold: #a88443;
  --rule: #dccfb4;
  --night: #1d1814;
  --night-text: #eadfca;
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0;
  background: var(--parchment);
  color: var(--ink);
  font-family: "EB Garamond", Georgia, serif;
  font-size: 21px;
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}
.layout {
  display: grid;
  grid-template-columns: 210px minmax(0, 800px);
  justify-content: center;
  gap: 64px;
}
main {
  min-width: 0;
  padding: 0 28px 140px;
}
h2, h3 { scroll-margin-top: 28px; }

/* The contents, at the side. */
.contents {
  position: sticky;
  top: 0;
  align-self: start;
  max-height: 100vh;
  overflow-y: auto;
  padding: 44px 0 40px 4px;
  display: flex;
  flex-direction: column;
  font-size: 16.5px;
  line-height: 1.3;
}
.contents a {
  padding: 7px 0 7px 14px;
  border-left: 2px solid var(--rule);
  color: var(--soft);
  text-decoration: none;
  transition: color 0.15s, border-color 0.15s;
}
.contents a:hover { color: var(--ink); }
.contents a.here {
  border-left-color: var(--rubric);
  color: var(--rubric);
  font-weight: 600;
}
.contents .home {
  margin-bottom: 14px;
  padding-left: 0;
  border: 0;
  font-family: "Cormorant Garamond", Georgia, serif;
  font-size: 30px;
  font-weight: 700;
  color: var(--rubric);
}
.contents .source {
  margin-top: 18px;
  padding-left: 0;
  border: 0;
  font-size: 13px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--gold);
}

/* Always at hand: the way to the install section. */
.try {
  position: fixed;
  right: 28px;
  bottom: 28px;
  z-index: 10;
  padding: 13px 26px 14px;
  border-radius: 999px;
  background: var(--rubric);
  color: var(--parchment);
  font-family: "Cormorant Garamond", Georgia, serif;
  font-size: 23px;
  font-weight: 700;
  letter-spacing: 0.01em;
  text-decoration: none;
  box-shadow: 0 0 0 1px #b8924d inset, 0 12px 28px -10px #4a0d0d;
  transition: transform 0.15s, background 0.15s;
}
.try:hover { background: #7a1616; transform: translateY(-2px); }
h1, h2, h3 { font-family: "Cormorant Garamond", Georgia, serif; line-height: 1.12; }

/* The opening: name, headline, and the line about the rooster. */
h1 {
  margin: 0;
  padding-top: 14vh;
  text-align: center;
  font-size: clamp(110px, 22vw, 220px);
  font-weight: 700;
  letter-spacing: -0.02em;
  color: var(--rubric);
}
h1::before {
  content: "A programming language";
  display: block;
  margin-bottom: 18px;
  font-family: "EB Garamond", Georgia, serif;
  font-size: 15px;
  font-weight: 500;
  letter-spacing: 0.32em;
  text-transform: uppercase;
  color: var(--gold);
}
h1 + h2 {
  margin: 10px 0 0;
  padding: 0;
  border: 0;
  text-align: center;
  font-size: clamp(30px, 5.2vw, 46px);
  font-style: italic;
  font-weight: 500;
  color: var(--ink);
}
h1 + h2::after {
  content: "";
  display: block;
  width: 84px;
  height: 1px;
  margin: 34px auto 0;
  background: var(--gold);
}
h1 + h2 + p {
  max-width: 560px;
  margin: 30px auto 70px;
  text-align: center;
  font-size: 23px;
  color: var(--soft);
}
h1 + h2 + p strong { font-weight: 500; }
h1 + h2 + p + p::first-letter {
  float: left;
  margin: 8px 12px 0 0;
  font-family: "Cormorant Garamond", Georgia, serif;
  font-size: 92px;
  font-weight: 700;
  line-height: 0.74;
  color: var(--rubric);
}

h2 {
  margin: 96px 0 26px;
  padding-top: 30px;
  border-top: 1px solid var(--rule);
  font-size: 40px;
  font-weight: 600;
  color: var(--rubric);
}
h3 { margin: 52px 0 14px; font-size: 29px; font-weight: 600; }
p { margin: 0 0 20px; }
a { color: var(--rubric); text-underline-offset: 3px; }
strong { font-weight: 600; }
ul, ol { margin: 0 0 22px; padding-left: 26px; }
li { margin-bottom: 8px; }
li::marker { color: var(--gold); }

code {
  font-family: "JetBrains Mono", ui-monospace, Menlo, monospace;
  font-size: 0.74em;
  padding: 2px 6px;
  border-radius: 4px;
  background: #ece2cc;
  color: #5c1414;
}
pre {
  margin: 0 0 26px;
  padding: 22px 24px;
  border-radius: 10px;
  overflow-x: auto;
  background: var(--night);
  color: var(--night-text);
  font-size: 15.5px;
  line-height: 1.62;
  box-shadow: 0 1px 0 #fff8 inset, 0 14px 34px -22px #2a1a0a;
}
pre code { padding: 0; background: none; color: inherit; font-size: inherit; }
pre.output, pre.report {
  background: var(--page);
  color: var(--ink);
  border: 1px solid var(--rule);
  border-left: 3px solid var(--gold);
  box-shadow: none;
}
pre.report { border-left-color: var(--rubric); }
pre.shell { background: #2a231d; }

/* Gesù code */
.k { color: #e0a36b; }
.t { color: #9fc1a6; }
.b { color: #d9c58a; }
.s { color: #b9c98f; }
.i { color: #f0d9a0; }
.e { color: #d99a6c; }
.n { color: #dba6a0; }
.c { color: #85776a; font-style: italic; }
.j { color: #f2cf6b; font-weight: 500; }
.f { color: #dba6a0; }
.h { color: #ff8f80; text-decoration: line-through; }
/* compiler reports */
.x { color: var(--rubric); font-weight: 500; }
.w { color: #96691a; font-weight: 500; }
.m { font-weight: 500; }
.d { color: #8a7c6c; }

.fact {
  display: grid;
  grid-template-columns: 64px 1fr;
  gap: 6px 18px;
  align-items: start;
  padding: 18px 0;
  border-bottom: 1px solid var(--rule);
}
.fact p { margin: 0; }
.numeral {
  font-family: "Cormorant Garamond", Georgia, serif;
  font-size: 50px;
  font-weight: 600;
  line-height: 1;
  text-align: right;
  color: var(--rubric);
}

.table { margin: 0 0 28px; overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 19px; }
th {
  padding: 10px 14px 10px 0;
  border-bottom: 2px solid var(--gold);
  text-align: left;
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--soft);
}
td { padding: 11px 14px 11px 0; border-bottom: 1px solid var(--rule); vertical-align: top; }

/* The last word. */
main > p:last-child {
  margin: 100px 0 0;
  text-align: center;
  font-family: "Cormorant Garamond", Georgia, serif;
  font-size: 64px;
  font-style: italic;
  font-weight: 500;
  color: var(--rubric);
}

@media (max-width: 1080px) {
  .layout { display: block; max-width: 856px; margin: 0 auto; }
  .contents { display: none; }
}
@media (max-width: 600px) {
  .try { right: 16px; bottom: 16px; font-size: 20px; padding: 11px 22px 12px; }
  body { font-size: 19px; }
  main { padding: 0 18px 80px; }
  h2 { font-size: 32px; margin-top: 70px; }
  pre { padding: 16px; font-size: 13.5px; border-radius: 8px; }
  .fact { grid-template-columns: 44px 1fr; gap: 6px 12px; }
  .numeral { font-size: 38px; }
}
</style>
</head>
<body>
<div class="layout">
<nav class="contents" aria-label="Contents">
<a class="home" href="#ges">Gesù</a>
__TOC__
<a class="source" href="https://github.com/lschiavini/gesu">Source on GitHub</a>
</nav>
<main>
__CONTENT__
</main>
</div>
<a class="try" href="#install">Try now</a>
<script>
// Marks, in the side contents, the section being read.
const links = [...document.querySelectorAll(".contents a[href^='#']")].filter((a) => !a.classList.contains("home"));
const byId = new Map(links.map((a) => [a.getAttribute("href").slice(1), a]));
const observer = new IntersectionObserver(
  (entries) => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        links.forEach((a) => a.classList.remove("here"));
        byId.get(entry.target.id)?.classList.add("here");
      }
    }
  },
  { rootMargin: "-10% 0px -80% 0px" }
);
byId.forEach((_, id) => {
  const heading = document.getElementById(id);
  if (heading) observer.observe(heading);
});
</script>
</body>
</html>
"""


def main():
    content, sections = render(README.read_text(encoding="utf-8"))
    toc = "\n".join(f'<a href="#{anchor}">{html.escape(label)}</a>' for anchor, label in sections)
    page = PAGE.replace("__CONTENT__", content).replace("__TOC__", toc)
    (ROOT / "index.html").write_text(page, encoding="utf-8")
    print(f"wrote {ROOT / 'index.html'}")


if __name__ == "__main__":
    main()
