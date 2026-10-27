// Help terms, the evidence highlight, and the copy button.
// Every explanation is read from the glossary on the page (#def-<term>), so a
// term has one definition wherever it appears and the page reads fine without JS.

document.documentElement.classList.add("js");

const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/* Help terms */

const tip = document.getElementById("tip");
let current = null;
let closeTimer = 0;

function definition(term) {
  const dd = document.getElementById("def-" + term.dataset.term);
  return dd ? dd.textContent.trim() : "";
}

function place(term) {
  const r = term.getBoundingClientRect();
  const block = term.closest("p, td, li, dd") || term.parentElement;
  const b = block.getBoundingClientRect();
  const vw = document.documentElement.clientWidth;
  const vh = window.innerHeight;
  const w = tip.offsetWidth;
  const h = tip.offsetHeight;
  const gap = 18;

  // In the margin to the left, when the term starts its line and the margin has room.
  if (r.left - b.left < 8 && b.left - w - gap >= 8 && vw >= 900) {
    const top = Math.max(8, r.top + r.height / 2 - 34);
    tip.dataset.side = "left";
    tip.style.left = `${window.scrollX + r.left - w - gap}px`;
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

function open(term) {
  clearTimeout(closeTimer);
  if (current && current !== term) current.setAttribute("aria-expanded", "false");
  current = term;
  term.setAttribute("aria-expanded", "true");
  tip.textContent = definition(term);
  tip.hidden = false;
  place(term);
  void tip.offsetWidth; // commit the closed state so the opening transition runs
  tip.classList.add("is-open");
}

function close() {
  clearTimeout(closeTimer);
  if (!current) return;
  current.setAttribute("aria-expanded", "false");
  current = null;
  tip.classList.remove("is-open");
  tip.hidden = true;
}

function closeSoon() {
  clearTimeout(closeTimer);
  closeTimer = setTimeout(close, 160);
}

for (const term of document.querySelectorAll(".term")) {
  term.setAttribute("aria-expanded", "false");
  term.addEventListener("pointerenter", (e) => { if (e.pointerType === "mouse") open(term); });
  term.addEventListener("pointerleave", (e) => { if (e.pointerType === "mouse") closeSoon(); });
  term.addEventListener("focus", () => open(term));
  term.addEventListener("blur", closeSoon);
  term.addEventListener("click", (e) => {
    e.stopPropagation();
    if (current === term && e.pointerType !== "mouse") close();
    else open(term);
  });
}

tip.addEventListener("pointerenter", () => clearTimeout(closeTimer));
tip.addEventListener("pointerleave", closeSoon);
document.addEventListener("keydown", (e) => { if (e.key === "Escape" && current) close(); });
document.addEventListener("click", (e) => { if (current && !tip.contains(e.target)) close(); });
window.addEventListener("resize", () => { if (current) place(current); });

// ?explain=<term> opens that term's explanation, so a definition can be linked to.
const asked = new URLSearchParams(location.search).get("explain");
if (asked) {
  const term = document.querySelector(`.term[data-term="${CSS.escape(asked)}"]`);
  if (term) document.fonts.ready.then(() => open(term));
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
  }, { threshold: 0.6 });
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
