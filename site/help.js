// Help terms, the evidence filter, and the copy button.
// Every explanation is read from the hidden definitions on the page (#def-<term>),
// which match glossary.html, so a term reads the same wherever it appears.

document.documentElement.classList.add("js");


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
// An unknown term goes to the glossary page instead of doing nothing.
const asked = new URLSearchParams(location.search).get("explain");
if (asked) {
  const term = document.querySelector(`.term[data-term="${CSS.escape(asked)}"]`);
  document.fonts.ready.then(() => {
    if (term) open(term, "link");
    else location.assign("glossary.html");
  });
}

/* Sections and evidence entries are <details>, so they open without JavaScript.
   A link to one (#panel-finding, #witness) opens it and any entry it sits in. */

function openTarget(hash) {
  const target = hash.length > 1 && document.getElementById(decodeURIComponent(hash.slice(1)));
  if (!target) return null;
  for (let el = target; el; el = el.parentElement.closest("details")) {
    if (el.tagName === "DETAILS") el.open = true;
    if (!el.parentElement) break;
  }
  return target;
}

/* Questions filter the evidence, and the strength filter narrows it further.
   Without JavaScript every entry shows and each question is a jump link. */

const blocks = [...document.querySelectorAll(".block")];
const groups = [...document.querySelectorAll(".group")];
const questionLinks = [...document.querySelectorAll("[data-question]")];
const strengthButtons = [...document.querySelectorAll(".strengths button")];
const answer = document.getElementById("answer");
const filter = { question: "", strength: "" };
// The one name each strength goes by wherever the script writes it. index.html uses the same names.
const STRENGTH_NAMES = {
  proved: "Proved",
  tested: "Tested",
  decided: "Decided by Harry",
  assumed: "Assumed",
  unreached: "Not yet reached",
};
const QUESTION_KEYS = new Set(questionLinks.map((l) => l.dataset.question));

// The URL fragment is the filter's one serialization: "", "#q-<question>", "#s=<strength>"
// or both joined by "&".
function serialize({ question, strength }) {
  const parts = [question && `q-${question}`, strength && `s=${strength}`].filter(Boolean);
  return parts.length ? `#${parts.join("&")}` : "";
}

// A fragment that is neither filter (#witness) is not unknown, it is a plain link target.
function parse(hash) {
  const next = { question: "", strength: "", unknown: false };
  if (!/^#(q-|s=)/.test(hash)) return next;
  for (const part of hash.slice(1).split("&")) {
    if (part.startsWith("q-") && !next.question && QUESTION_KEYS.has(part.slice(2))) next.question = part.slice(2);
    else if (part.startsWith("s=") && !next.strength && Object.hasOwn(STRENGTH_NAMES, part.slice(2))) next.strength = part.slice(2);
    else next.unknown = true;
  }
  return next;
}

if (blocks.length && answer) {
  document.querySelector(".strengths").hidden = false;
  for (const button of strengthButtons) {
    const label = button.querySelector("span");
    label.textContent = STRENGTH_NAMES[button.dataset.strength];
    // The visible count is aria-hidden in the markup. This suffix sits inside the label, because
    // flex items are read as separate words, and gives "Proved, 2 pieces" instead of "Proved2".
    const spoken = document.createElement("span");
    spoken.className = "visually-hidden strength-spoken";
    label.append(spoken);
  }
  // The q-anchor ids exist only for the jump without JavaScript. With it, the browser's own
  // fragment scroll would run after the script anchors and undo it.
  for (const span of document.querySelectorAll(".q-anchor")) span.removeAttribute("id");
  history.scrollRestoration = "manual";
  const status = document.querySelector(".filter-status");
  // A bad link is reported under the Questions label, which stays on screen at every width.
  const notice = document.querySelector(".filter-notice");
  const title = document.getElementById("answer-title");
  const text = answer.querySelector(".answer-text");
  const count = answer.querySelector(".answer-count");
  const showAll = answer.querySelector(".show-all");
  const words = (value) => (value || "").split(" ").filter(Boolean);
  const strengthName = (key) => STRENGTH_NAMES[key];
  const pieces = (n) => (n === 1 ? "1 piece" : `${n} pieces`);

  // A chosen question opens the evidence that answers it. The entries the filter opened are
  // remembered, so clearing the question closes those and leaves any the reader opened.
  const openedByFilter = new Set();
  const offered = new Set();
  let offeredFor = "";
  function openForQuestion() {
    if (!filter.question) {
      for (const block of openedByFilter) block.open = false;
      openedByFilter.clear();
    }
    if (offeredFor !== filter.question) { offered.clear(); offeredFor = filter.question; }
    if (!filter.question) return;
    for (const block of blocks) {
      if (block.hidden || offered.has(block)) continue;
      offered.add(block);
      if (!block.open) { block.open = true; openedByFilter.add(block); }
    }
  }

  function apply(unknown = false) {
    let shown = 0;
    const perStrength = Object.fromEntries(Object.keys(STRENGTH_NAMES).map((key) => [key, 0]));
    for (const block of blocks) {
      const inQuestion = !filter.question || words(block.dataset.q).includes(filter.question);
      if (inQuestion) for (const key of words(block.dataset.strength)) perStrength[key] += 1;
      const visible = inQuestion && (!filter.strength || words(block.dataset.strength).includes(filter.strength));
      block.hidden = !visible;
      if (visible) shown += 1;
    }
    for (const group of groups) group.hidden = !group.querySelector(".block:not([hidden])");
    for (const link of questionLinks) {
      const chosen = link.dataset.question === filter.question;
      const hint = link.querySelector(".chosen-hint");
      if (chosen) {
        link.setAttribute("aria-current", "true");
        if (!hint) link.append(chosenHint());
      } else {
        link.removeAttribute("aria-current");
        if (hint) hint.remove();
      }
    }
    for (const button of strengthButtons) {
      const key = button.dataset.strength;
      const pressed = key === filter.strength;
      button.setAttribute("aria-pressed", String(pressed));
      // Counts warn of a dead end, which only exists once a question narrows the list.
      button.querySelector(".strength-count").textContent = filter.question ? perStrength[key] : "";
      button.querySelector(".strength-spoken").textContent = filter.question ? `, ${pieces(perStrength[key])}` : "";
      button.toggleAttribute("data-empty", Boolean(filter.question) && perStrength[key] === 0 && !pressed);
    }

    openForQuestion();
    const marked = filter.strength ? ` marked ${strengthName(filter.strength)}` : "";
    answer.hidden = !filter.question;
    if (filter.question) {
      title.textContent = questionLinks.find((l) => l.dataset.question === filter.question).querySelector("span").textContent;
      text.innerHTML = document.querySelector(`[data-answer="${filter.question}"]`).innerHTML;
      const empty = shown === 0;
      count.textContent = empty
        ? `None of the evidence for this question is${marked}.`
        : `Showing ${pieces(shown)} of evidence${marked}, out of ${blocks.length}.`;
      showAll.textContent = empty ? "Clear the strength filter" : "Show everything";
    }
    notice.textContent = unknown ? "That link names a question or strength this page doesn\u2019t have. Pick a question from the list." : "";
    status.textContent = !filter.question && filter.strength ? `Showing ${pieces(shown)} of evidence${marked}, out of ${blocks.length}.` : "";
  }

  function chosenHint() {
    const hint = document.createElement("span");
    hint.className = "visually-hidden chosen-hint";
    hint.textContent = " (chosen; choose it again to clear)";
    return hint;
  }

  // Every filter change leaves the filters where they were: their top edge at the top of the
  // screen, with the answer and evidence below. Stacked on a phone they fill the screen,
  // so there the view anchors to what the filter produced instead.
  const filters = document.getElementById("questions");
  const wide = window.matchMedia("(min-width: 861px)");
  function anchor() {
    const target = wide.matches ? filters : (filter.question ? answer : document.getElementById("evidence"));
    // Pad the page by exactly what the target needs to reach the top of the screen.
    document.body.style.paddingBottom = "";
    const top = target.getBoundingClientRect().top + window.scrollY;
    const needed = Math.ceil(top - (document.documentElement.scrollHeight - window.innerHeight));
    if (needed > 0) document.body.style.paddingBottom = `${needed}px`;
    window.scrollTo({ top, behavior: "instant" });
  }

  // Every filter change goes through here: the state, the view, the history entry and the scroll.
  function update(next, { push = true, scroll = true, unknown = false } = {}) {
    const questionChanged = next.question !== filter.question;
    filter.question = next.question;
    filter.strength = next.strength;
    apply(unknown);
    if (push) history.pushState(null, "", serialize(filter) || location.pathname + location.search);
    if (filter.question && questionChanged) title.focus({ preventScroll: true });
    if (scroll) anchor();
  }

  for (const link of questionLinks) {
    link.addEventListener("click", (e) => {
      e.preventDefault();
      update({ question: filter.question === link.dataset.question ? "" : link.dataset.question, strength: filter.strength });
    });
  }
  // A strength works like a document type on the archive: press it to filter, press again to clear.
  for (const button of strengthButtons) {
    button.addEventListener("click", () => {
      update({ question: filter.question, strength: filter.strength === button.dataset.strength ? "" : button.dataset.strength });
    });
  }
  showAll.addEventListener("click", () => {
    const strengthOnly = showAll.textContent === "Clear the strength filter";
    update({ question: strengthOnly ? filter.question : "", strength: "" });
  });

  // A link into the page clears any filter hiding its target, then opens it.
  // A link back to the filters only scrolls: a fragment jump would reach popstate and clear the filter.
  document.addEventListener("click", (e) => {
    const link = e.target.closest('a[href^="#"]');
    if (!link || link.dataset.question) return;
    if (link.getAttribute("href") === "#questions") {
      e.preventDefault();
      filters.scrollIntoView({ behavior: "instant" });
      return;
    }
    const target = document.getElementById(link.getAttribute("href").slice(1));
    if (target && target.closest("[hidden]")) update({ question: "", strength: "" }, { push: false, scroll: false });
    if (target) openTarget(link.getAttribute("href"));
  });

  const fromHash = ({ scroll = true } = {}) => {
    const next = parse(location.hash);
    const filtered = Boolean(next.question || next.strength);
    if (next.question !== filter.question || next.strength !== filter.strength || next.unknown) {
      update(next, { push: false, scroll: scroll && filtered, unknown: next.unknown });
    }
    if (!filtered) {
      const target = openTarget(location.hash);
      if (target) target.scrollIntoView();
      else if (next.unknown && scroll) filters.scrollIntoView({ behavior: "instant" });
    }
  };
  window.addEventListener("popstate", () => fromHash());
  apply();
  fromHash({ scroll: false });
  // Anchor once the page has its final layout, after the load event and a frame.
  const settle = () => requestAnimationFrame(() => {
    if (filter.question || filter.strength) anchor();
    else if (notice.textContent) filters.scrollIntoView({ behavior: "instant" });
  });
  if (document.readyState === "complete") settle();
  else window.addEventListener("load", settle, { once: true });
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
