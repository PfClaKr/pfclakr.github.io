// Theme toggle, header behaviour, copy buttons, TOC, reading progress and giscus comments.
(function () {
  "use strict";
  var root = document.documentElement;
  var theme = function () { return root.dataset.theme === "dark" ? "dark" : "light"; };

  // ── Theme toggle ──
  function setTheme(t) {
    root.dataset.theme = t;
    try { localStorage.setItem("theme", t); } catch (e) {}
    document.dispatchEvent(new CustomEvent("themechange", { detail: t }));
    var frame = document.querySelector("iframe.giscus-frame");
    if (frame) frame.contentWindow.postMessage({ giscus: { setConfig: { theme: t } } }, "https://giscus.app");
  }
  var toggle = document.querySelector(".theme-toggle");
  if (toggle) toggle.addEventListener("click", function () { setTheme(theme() === "dark" ? "light" : "dark"); });

  // Close the language menu when clicking elsewhere.
  document.addEventListener("click", function (e) {
    document.querySelectorAll("details.lang-menu[open]").forEach(function (d) {
      if (!d.contains(e.target)) d.removeAttribute("open");
    });
  });

  // ── Code blocks: header bar with language label and copy button ──
  var copyText = document.body.dataset.copy || "Copy";
  var copiedText = document.body.dataset.copied || "Copied";
  document.querySelectorAll(".prose pre.code").forEach(function (pre) {
    var wrap = document.createElement("div");
    wrap.className = "code-wrap";
    pre.parentNode.insertBefore(wrap, pre);
    var bar = document.createElement("div");
    bar.className = "code-bar";
    var lang = document.createElement("span");
    lang.className = "code-lang";
    lang.textContent = pre.dataset.lang || "";
    bar.appendChild(lang);
    if (navigator.clipboard) {
      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "copy-btn";
      btn.textContent = copyText;
      btn.addEventListener("click", function () {
        navigator.clipboard.writeText(pre.innerText.replace(/\n$/, "")).then(function () {
          btn.textContent = copiedText;
          btn.classList.add("done");
          setTimeout(function () { btn.textContent = copyText; btn.classList.remove("done"); }, 1400);
        });
      });
      bar.appendChild(btn);
    }
    wrap.appendChild(bar);
    wrap.appendChild(pre);
    pre.classList.add("wrapped");
  });

  // ── Post list filter ──
  var filter = document.querySelector(".post-filter");
  if (filter) {
    var input = filter.querySelector("input");
    var cards = Array.prototype.slice.call(document.querySelectorAll(".post-card"));
    var groups = document.querySelectorAll(".year-group");
    var empty = document.querySelector(".filter-empty");
    var norm = function (t) { return t.toLowerCase().normalize("NFKD").replace(/[\u0300-\u036f]/g, ""); };
    var index = cards.map(function (c) { return norm(c.textContent); });
    filter.hidden = false;
    input.addEventListener("input", function () {
      var terms = norm(input.value.trim()).split(/\s+/).filter(Boolean);
      var shown = 0;
      cards.forEach(function (c, i) {
        var ok = terms.every(function (t) { return index[i].indexOf(t) !== -1; });
        c.hidden = !ok;
        if (ok) shown++;
      });
      groups.forEach(function (g) { g.hidden = !g.querySelector(".post-card:not([hidden])"); });
      if (empty) empty.hidden = shown !== 0;
    });
    input.addEventListener("keydown", function (e) {
      if (e.key === "Escape") { input.value = ""; input.dispatchEvent(new Event("input")); input.blur(); }
    });
    document.addEventListener("keydown", function (e) {
      if (e.key === "/" && !e.metaKey && !e.ctrlKey && !e.altKey && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) {
        e.preventDefault(); input.focus();
      }
    });
  }

  // ── TOC: expanded as a sidebar on wide screens, collapsed inline otherwise ──
  var tocBox = document.querySelector(".toc details");
  if (tocBox) {
    var wide = matchMedia("(min-width: 1280px)");
    var syncToc = function () { tocBox.open = wide.matches; };
    syncToc();
    if (wide.addEventListener) wide.addEventListener("change", syncToc);
    tocBox.addEventListener("click", function (e) {
      if (!wide.matches && e.target.closest("a")) tocBox.open = false;
    });
  }

  // ── Header: hide while scrolling down on small screens; reading progress ──
  var header = document.querySelector(".site-header");
  var bar = document.querySelector(".read-progress");
  var article = document.querySelector(".post .prose");
  var toTop = document.querySelector(".to-top");
  if (toTop) toTop.addEventListener("click", function () {
    var smooth = !matchMedia("(prefers-reduced-motion: reduce)").matches;
    window.scrollTo({ top: 0, behavior: smooth ? "smooth" : "auto" });
    var h = document.querySelector(".post-header h1");
    if (h) { h.setAttribute("tabindex", "-1"); h.focus({ preventScroll: true }); }
  });
  if (header) {
    var lastY = window.scrollY, ticking = false;
    var onScroll = function () {
      ticking = false;
      var y = window.scrollY;
      var hide = y > lastY && y > header.offsetHeight + 40 && !header.contains(document.activeElement);
      if (Math.abs(y - lastY) > 4) { header.classList.toggle("is-hidden", hide); lastY = y; }
      if (toTop) toTop.hidden = y < window.innerHeight * 1.5;
      if (bar && article) {
        var r = article.getBoundingClientRect();
        var total = r.height - window.innerHeight * 0.6;
        var p = total > 0 ? Math.min(1, Math.max(0, -r.top / total)) : 1;
        bar.style.transform = "scaleX(" + p + ")";
      }
    };
    window.addEventListener("scroll", function () {
      if (!ticking) { ticking = true; requestAnimationFrame(onScroll); }
    }, { passive: true });
    onScroll();
  }

  // ── Highlight the current section in the TOC ──
  var tocLinks = document.querySelectorAll(".toc a");
  if (tocLinks.length && "IntersectionObserver" in window) {
    var byId = {};
    tocLinks.forEach(function (a) { byId[decodeURIComponent(a.hash.slice(1))] = a; });
    var obs = new IntersectionObserver(function (entries) {
      entries.forEach(function (en) {
        if (!en.isIntersecting) return;
        tocLinks.forEach(function (a) { a.classList.remove("active"); });
        var a = byId[en.target.id];
        if (a) a.classList.add("active");
      });
    }, { rootMargin: "-80px 0px -70% 0px" });
    Object.keys(byId).forEach(function (id) {
      var h = document.getElementById(id);
      if (h) obs.observe(h);
    });
  }

  // ── giscus comments (GitHub Discussions) ──
  var box = document.getElementById("giscus");
  if (box) {
    var s = document.createElement("script");
    s.src = "https://giscus.app/client.js";
    Object.keys(box.dataset).forEach(function (k) { s.dataset[k] = box.dataset[k]; });
    s.dataset.theme = theme();
    s.crossOrigin = "anonymous";
    s.async = true;
    box.appendChild(s);
  }
})();
