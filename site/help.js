// Help terms, the evidence highlight, and the copy button.
// Every explanation is read from the glossary on the page (#def-<term>), so a
// term has one definition wherever it appears and the page reads fine without JS.

document.documentElement.classList.add("js");

const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/* Help terms */

const tip = document.getElementById("tip");
// The open term and how it was opened: "hover" | "focus" | "click" | "link".
// A click pins a term, so the focus that arrives with a tap or Enter cannot close it.
let state = null;
let closeTimer = 0;

function definition(term) {
  const dd = document.getElementById("def-" + term.dataset.term);
  return dd ? dd.textContent.trim() : "";
}

function place(term) {
  const r = term.getBoundingClientRect();
  const gloss = term.closest("[data-gloss]");
  const block = gloss || term.closest("p, td, li, dd") || term.parentElement;
  const b = block.getBoundingClientRect();
  const vw = document.documentElement.clientWidth;
  const vh = window.innerHeight;
  const w = tip.offsetWidth;
  const h = tip.offsetHeight;
  const gap = 18;

  // In the left margin, for a term inside an element that opts in and has the room.
  if (gloss && gloss.dataset.gloss === "margin" && b.left - w - gap >= 8 && vw >= 900) {
    const top = Math.max(8, r.top + r.height / 2 - 34);
    tip.dataset.side = "left";
    tip.style.left = `${window.scrollX + b.left - w - gap}px`;
    tip.style.top = `${window.scrollY + top}px`;
    tip.style.setProperty("--arrow", `${r.top + r.height / 2 - top}px`);
    return;
  }

  // Otherwise below the term, or above it when there is no room below.
  const left = Math.min(Math.max(8, r.left + r.width / 2 - 28), vw - w - 8);
  const below = r.bottom + 14 + h <= vh || r.top - 14 - h < 0;
  tip.dataset.side = below ? "below" : "above";
  tip.style.left = `${window.scrollX + left}px`;
  tip.style.top = `${window.scrollY + (below ? r.bottom + 14 : r.top - 14 - h)}px`;
  tip.style.setProperty("--arrow", `${r.left + r.width / 2 - left}px`);
}

function open(term, via) {
  clearTimeout(closeTimer);
  if (state && state.term !== term) delete state.term.dataset.open;
  state = { term, via };
  term.dataset.open = "";
  tip.textContent = definition(term);
  tip.hidden = false;
  place(term);
  void tip.offsetWidth; // commit the closed state so the opening transition runs
  tip.classList.add("is-open");
}

function close() {
  clearTimeout(closeTimer);
  if (!state) return;
  delete state.term.dataset.open;
  state = null;
  tip.classList.remove("is-open");
  tip.hidden = true;
}

// Closes after a short delay, but only a tip that was opened in one of the given ways.
function closeSoon(vias) {
  clearTimeout(closeTimer);
  closeTimer = setTimeout(() => { if (state && vias.includes(state.via)) close(); }, 160);
}

const pinned = (term) => state !== null && state.term === term && state.via === "click";

for (const term of document.querySelectorAll(".term")) {
  term.addEventListener("pointerenter", (e) => { if (e.pointerType === "mouse" && !pinned(term)) open(term, "hover"); });
  term.addEventListener("pointerleave", (e) => { if (e.pointerType === "mouse") closeSoon(["hover"]); });
  term.addEventListener("focus", () => { if (!pinned(term)) open(term, "focus"); });
  term.addEventListener("blur", () => closeSoon(["hover", "focus", "link"]));
  term.addEventListener("click", (e) => {
    e.stopPropagation();
    if (pinned(term)) close();
    else open(term, "click");
  });
}

tip.addEventListener("pointerenter", () => clearTimeout(closeTimer));
tip.addEventListener("pointerleave", () => closeSoon(["hover"]));
document.addEventListener("keydown", (e) => { if (e.key === "Escape" && state) close(); });
document.addEventListener("click", (e) => { if (state && !tip.contains(e.target)) close(); });
window.addEventListener("resize", () => { if (state) place(state.term); });

// ?explain=<term> opens that term's explanation, so a definition can be linked to.
// An unknown term lands on the glossary instead of doing nothing.
const asked = new URLSearchParams(location.search).get("explain");
if (asked) {
  const term = document.querySelector(`.term[data-term="${CSS.escape(asked)}"]`);
  document.fonts.ready.then(() => {
    if (term) open(term, "link");
    else document.getElementById("terms")?.scrollIntoView();
  });
}

/* The finding is marked as it scrolls into view. If it is already on screen
   when the page loads, the mark is simply there. */

const finding = document.querySelector(".finding");
if (finding && "IntersectionObserver" in window && !reduceMotion) {
  let first = true;
  const io = new IntersectionObserver((entries) => {
    const visible = entries.some((e) => e.isIntersecting);
    if (first) {
      first = false;
      if (visible) { io.disconnect(); return; }
      finding.classList.add("is-waiting");
      return;
    }
    if (!visible) return;
    io.disconnect();
    finding.classList.add("is-marked");
    void finding.offsetWidth;
    finding.classList.remove("is-waiting");
  }, { threshold: 0.35 });
  io.observe(finding);
}

/* Copy the install commands */

for (const button of document.querySelectorAll(".copy")) {
  const status = button.parentElement.querySelector(".copy-status");
  let resetTimer = 0;
  button.addEventListener("click", async () => {
    const text = document.getElementById(button.dataset.copy).textContent;
    clearTimeout(resetTimer);
    try {
      await navigator.clipboard.writeText(text);
      status.textContent = "Copied.";
    } catch {
      status.textContent = "Could not copy. Select the commands and copy them.";
    }
    resetTimer = setTimeout(() => { status.textContent = ""; }, 3000);
  });
}
