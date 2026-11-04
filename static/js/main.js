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

  // ── Copy buttons on code blocks ──
  document.querySelectorAll(".prose pre.code").forEach(function (pre) {
    if (!navigator.clipboard) return;
    var wrap = document.createElement("div");
    wrap.className = "code-wrap";
    pre.parentNode.insertBefore(wrap, pre);
    wrap.appendChild(pre);
    var btn = document.createElement("button");
    btn.type = "button";
    btn.className = "copy-btn";
    btn.textContent = "Copy";
    btn.addEventListener("click", function () {
      navigator.clipboard.writeText(pre.innerText).then(function () {
        btn.textContent = "Copied";
        btn.classList.add("done");
        setTimeout(function () { btn.textContent = "Copy"; btn.classList.remove("done"); }, 1400);
      });
    });
    wrap.appendChild(btn);
  });

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
  if (header) {
    var lastY = window.scrollY, ticking = false;
    var onScroll = function () {
      ticking = false;
      var y = window.scrollY;
      var hide = y > lastY && y > header.offsetHeight + 40 && !header.contains(document.activeElement);
      if (Math.abs(y - lastY) > 4) { header.classList.toggle("is-hidden", hide); lastY = y; }
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
