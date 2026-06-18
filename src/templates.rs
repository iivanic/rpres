//! Embedded HTML templates, themes, shared layout CSS and navigation JS.

/// Names of the available templates, in display order.
pub const TEMPLATES: [&str; 3] = ["terminal", "classic", "modern"];

/// Returns the theme CSS for `name`, or `None` if the template is unknown.
pub fn theme_css(name: &str) -> Option<&'static str> {
    match name {
        "terminal" => Some(TERMINAL_CSS),
        "classic" => Some(CLASSIC_CSS),
        "modern" => Some(MODERN_CSS),
        _ => None,
    }
}

/// HTML skeleton with `{{PLACEHOLDER}}` markers filled in by the renderer.
pub const SKELETON: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{{TITLE}}</title>
<style>
{{BASE_CSS}}
{{THEME_CSS}}
</style>
</head>
<body data-anim="{{ANIM}}" data-paged="{{PAGED}}" data-count="{{COUNT}}">
<div id="deck">{{DECK}}</div>
<div id="hud"><span id="pageno"></span></div>
<script>
{{JS}}
</script>
</body>
</html>
"#;

/// Layout CSS shared by every theme.
pub const BASE_CSS: &str = r#"
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; overflow: hidden; }
#deck { position: relative; width: 100vw; height: 100vh; }
.slide {
    display: none;
    position: absolute;
    inset: 0;
    padding: 6vh 8vw;
    flex-direction: column;
    justify-content: flex-start;
    overflow: hidden;
}
.slide.active { display: flex; }
.slide.title {
    justify-content: center;
    align-items: center;
    text-align: center;
}
/* The body is shrunk by JS (PowerPoint-style autofit) when its content is too
   tall to fit. The scale is a ratio of measured pixels, so it is unaffected by
   browser zoom. */
.slide-body { width: 100%; transform-origin: top left; }
.slide.title .slide-body { transform-origin: top center; }
.fragment { transition: opacity .25s ease; }
body.anim .fragment:not(.revealed) { opacity: 0; pointer-events: none; }
body.anim li.fragment:not(.revealed) { list-style: none; }
#hud {
    position: fixed;
    bottom: 1.5vh;
    right: 2vw;
    font-size: 1.6vh;
    opacity: .55;
    user-select: none;
}
img { max-width: 100%; height: auto; }
pre { overflow: auto; }
table { border-collapse: collapse; }
/* Draw task-list checkboxes ourselves (appearance:none) so they are sized in em
   and stay a fixed physical size under browser zoom. Native widgets (notably in
   Firefox) ignore em sizing and scale with zoom. */
input[type="checkbox"] {
    appearance: none; -webkit-appearance: none;
    box-sizing: border-box;
    font: inherit;
    width: 1em; height: 1em; margin: 0 .45em 0 0;
    vertical-align: -0.1em;
    border: .12em solid currentColor; border-radius: .18em;
    background: transparent; position: relative; flex: none; cursor: default;
}
input[type="checkbox"]:checked::after {
    content: ""; position: absolute; box-sizing: border-box;
    left: .28em; top: .06em; width: .28em; height: .55em;
    border: solid currentColor; border-width: 0 .16em .16em 0;
    transform: rotate(45deg);
}
/* Indent lists in em (not the browser-default ~40px) so indentation tracks the
   font size and stays a fixed physical size under browser zoom. */
ul, ol { padding-left: 1.4em; margin: .5em 0; }
li { margin: .15em 0; }

/* Print / "Save as PDF": one slide per page at 16:9, reveal everything. */
@page { size: 297mm 167mm; margin: 0; }
@media print {
    html, body { height: auto; overflow: visible; }
    /* The theme paints its background on <body>, which the browser does not
       repeat per page. Re-apply it to every slide so each PDF page matches. */
    html { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
    #deck { position: static; width: auto; height: auto; background: inherit; }
    .slide {
        display: flex !important;
        position: relative;
        inset: auto;
        width: 297mm;
        height: 167mm;
        overflow: hidden;
        background: inherit;
        break-after: page;
        break-inside: avoid;
        -webkit-print-color-adjust: exact;
        print-color-adjust: exact;
    }
    .slide:last-child { break-after: auto; }
    body.anim .fragment:not(.revealed) { opacity: 1 !important; }
    body.anim li.fragment:not(.revealed) { list-style: revert; }
    #hud { display: none; }
}
"#;

/// Navigation / animation JavaScript shared by every template.
pub const JS: &str = r#"
(function () {
  const body = document.body;
  const deck = document.getElementById('deck');
  const hud = document.getElementById('pageno');
  let anim = body.dataset.anim === '1';
  const paged = body.dataset.paged === '1';
  const count = parseInt(body.dataset.count || '0', 10);
  let idx = 0;
  let frag = 0;

  function applyAnimClass() { body.classList.toggle('anim', anim); }
  function curSlide() {
    return deck.querySelector('.slide.active') || deck.querySelector('.slide');
  }
  function fragEls() {
    const s = curSlide();
    return s ? s.querySelectorAll('.fragment') : [];
  }
  function renderFrags() {
    const els = fragEls();
    els.forEach((f, k) => f.classList.toggle('revealed', !anim || k < frag));
  }
  // PowerPoint-style autofit: scale the active slide's body down until it fits
  // the slide. Capped at 1 so text never grows past its natural size. Because
  // the scale is a ratio of measured pixels, the result is the same at any
  // browser zoom level.
  function fitSlide() {
    const s = curSlide();
    if (!s) return;
    const bodyEl = s.querySelector('.slide-body');
    if (!bodyEl) return;
    bodyEl.style.transform = 'none';
    const cs = getComputedStyle(s);
    const padT = parseFloat(cs.paddingTop) || 0;
    const padB = parseFloat(cs.paddingBottom) || 0;
    const padL = parseFloat(cs.paddingLeft) || 0;
    const padR = parseFloat(cs.paddingRight) || 0;
    const head = s.querySelector('.slide-head');
    const headH = head ? head.offsetHeight : 0;
    const availH = s.clientHeight - padT - padB - headH;
    const availW = s.clientWidth - padL - padR;
    const ch = bodyEl.scrollHeight;
    const cw = bodyEl.scrollWidth;
    let scale = Math.min(1, availH / ch, availW / cw);
    if (!isFinite(scale) || scale <= 0) scale = 1;
    bodyEl.style.transform = scale < 1 ? 'scale(' + scale + ')' : 'none';
  }
  function scheduleFit() { requestAnimationFrame(fitSlide); }
  function updateHud() { hud.textContent = (idx + 1) + ' / ' + count; }
  function activate(i) {
    const slides = deck.querySelectorAll('.slide');
    slides.forEach((s, k) => s.classList.toggle('active', paged ? true : k === i));
    renderFrags();
    updateHud();
    scheduleFit();
  }
  async function loadPaged(i) {
    const res = await fetch('/slide/' + i);
    deck.innerHTML = await res.text();
  }
  async function go(i, revealAll) {
    if (i < 0 || i >= count) return;
    idx = i;
    if (paged) await loadPaged(i);
    activate(i);
    const total = fragEls().length;
    frag = revealAll ? total : 0;
    renderFrags();
  }
  function next() {
    const total = fragEls().length;
    if (anim && frag < total) { frag++; renderFrags(); return; }
    go(idx + 1, false);
  }
  function prev() {
    if (anim && frag > 0) { frag--; renderFrags(); return; }
    go(idx - 1, true);
  }

  document.addEventListener('keydown', (e) => {
    switch (e.key) {
      case 'ArrowRight':
      case 'PageDown':
      case ' ':
      case 'Enter':
      case 'l':
        e.preventDefault(); next(); break;
      case 'ArrowLeft':
      case 'PageUp':
      case 'Backspace':
      case 'h':
        e.preventDefault(); prev(); break;
      case 'Home': e.preventDefault(); go(0, false); break;
      case 'End': e.preventDefault(); go(count - 1, true); break;
      case 'a':
      case 'A':
        anim = !anim; applyAnimClass();
        frag = anim ? 0 : fragEls().length; renderFrags(); break;
      case 'f':
      case 'F':
        if (!document.fullscreenElement) document.documentElement.requestFullscreen();
        else document.exitFullscreen();
        break;
    }
  });
  document.addEventListener('click', (e) => {
    if (e.target.closest('a')) return;
    next();
  });

  // Re-fit on viewport changes and once images finish loading (load does not
  // bubble, so listen in the capture phase).
  window.addEventListener('resize', scheduleFit);
  deck.addEventListener('load', (e) => {
    if (e.target && e.target.tagName === 'IMG') scheduleFit();
  }, true);

  applyAnimClass();
  go(0, false);
})();
"#;

const TERMINAL_CSS: &str = r###"
body {
    background: #0b0e0b;
    color: #33ff66;
    font-family: "SFMono-Regular", "JetBrains Mono", Consolas, monospace;
    font-size: 2.6vh;
    line-height: 1.5;
}
.slide-head h1, .slide-head h2 { color: #7dffa6; margin: 0 0 .6em; }
a { color: #6cf; }
code { background: #052105; padding: 0 .3em; border-radius: 3px; }
pre { background: #041504; padding: 1em; border: 1px solid #1c3b1c; border-radius: 6px; }
pre code { background: none; padding: 0; }
blockquote { border-left: .18em solid #2a7a3f; margin: .5em 0; padding-left: 1em; color: #9bdca8; }
th, td { border: 1px solid #2a7a3f; padding: .3em .7em; }
#hud { color: #33ff66; }
"###;

const CLASSIC_CSS: &str = r#"
body {
    background: #fffdf7;
    color: #222;
    font-family: Georgia, "Times New Roman", serif;
    font-size: 2.7vh;
    line-height: 1.55;
}
.slide-head h1 { color: #8a1f2b; margin: 0 0 .4em; font-size: 2em; }
.slide-head h2 { color: #1f3a8a; margin: 0 0 .5em; border-bottom: 2px solid #d8d2c0; padding-bottom: .2em; }
a { color: #1f3a8a; }
code { background: #f0ece0; padding: 0 .3em; border-radius: 3px; font-family: Consolas, monospace; }
pre { background: #f4f0e6; padding: 1em; border: 1px solid #ddd6c4; border-radius: 4px; }
pre code { background: none; }
blockquote { border-left: .22em solid #c9b27a; margin: .5em 0; padding-left: 1em; font-style: italic; color: #555; }
th, td { border: 1px solid #ccc4ac; padding: .35em .8em; }
th { background: #efe9d8; }
#hud { color: #555; }
"#;

const MODERN_CSS: &str = r#"
body {
    background: radial-gradient(circle at 30% 20%, #1e293b, #0f172a 70%);
    color: #e5edf5;
    font-family: "Inter", "Segoe UI", system-ui, sans-serif;
    font-size: 2.7vh;
    line-height: 1.6;
}
.slide-head h1 {
    margin: 0 0 .3em;
    font-size: 2.4em;
    background: linear-gradient(90deg, #38bdf8, #a78bfa);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
}
.slide-head h2 { margin: 0 0 .5em; color: #7dd3fc; font-weight: 600; }
a { color: #7dd3fc; }
code { background: rgba(148,163,184,.18); padding: .05em .35em; border-radius: 4px; font-family: "JetBrains Mono", Consolas, monospace; }
pre { background: rgba(15,23,42,.7); padding: 1em; border: 1px solid rgba(148,163,184,.25); border-radius: 10px; }
pre code { background: none; padding: 0; }
blockquote { border-left: .22em solid #a78bfa; margin: .5em 0; padding-left: 1em; color: #c4b5fd; }
th, td { border: 1px solid rgba(148,163,184,.3); padding: .4em .8em; }
th { background: rgba(148,163,184,.12); }
#hud { color: #94a3b8; }
"#;
