// Theme toggle, header behaviour, copy buttons, TOC, reading progress and giscus comments.
(function () {
  "use strict";
  var root = document.documentElement;
  // Lowercase + strip Latin accents, one char in → one char out, so match
  // offsets can be used on the original string (NFKD would split Hangul).
  var norm = function (t) {
    return t.toLowerCase().replace(/[\u00c0-\u024f]/g, function (c) { return c.normalize("NFD").charAt(0); });
  };
  var escapeHtml = function (t) {
    return t.replace(/[&<>"]/g, function (c) { return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]; });
  };
  var theme = function () { return root.dataset.theme === "dark" ? "dark" : "light"; };

  // ── Theme toggle ──
  function setTheme(t) {
    root.dataset.theme = t;
    try { localStorage.setItem("theme", t); } catch (e) {}
    document.dispatchEvent(new CustomEvent("themechange", { detail: t }));
    var frame = document.querySelector("iframe.giscus-frame");
    if (frame) frame.contentWindow.postMessage({ giscus: { setConfig: { theme: t } } }, "https://giscus.app");
  }
  // Language dropdown: close on outside click or Escape.
  var langMenu = document.querySelector("details.lang-menu");
  if (langMenu) {
    document.addEventListener("click", function (e) { if (langMenu.open && !langMenu.contains(e.target)) langMenu.open = false; });
    langMenu.addEventListener("keydown", function (e) {
      if (e.key === "Escape" && langMenu.open) { langMenu.open = false; langMenu.querySelector("summary").focus(); }
    });
  }

  var toggle = document.querySelector(".theme-toggle");
  if (toggle) toggle.addEventListener("click", function () { setTheme(theme() === "dark" ? "light" : "dark"); });


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
  }

  // ── Site search (index loaded on first open) ──
  var dlg = document.querySelector(".search-dialog");
  if (dlg && typeof dlg.showModal === "function") {
    var sInput = dlg.querySelector("input");
    var list = dlg.querySelector(".search-results");
    var status = dlg.querySelector(".search-status");
    var emptyText = dlg.dataset.empty || "No results.";
    var docs = null, loading = null, sel = -1, results = [];

    var load = function () {
      if (!loading) {
        loading = fetch(dlg.dataset.index)
          .then(function (r) { return r.json(); })
          .then(function (d) {
            docs = d.map(function (e) {
              return { e: e, t: norm(e.t), g: norm(e.g.join(" ")), s: norm(e.s), x: norm(e.x) };
            });
          })
          .catch(function () { loading = null; });
      }
      return loading;
    };

    // Wrap every occurrence of the terms in <mark>, escaping the rest.
    var highlight = function (text, terms) {
      var n = norm(text), ranges = [];
      terms.forEach(function (t) {
        for (var i = n.indexOf(t); i !== -1; i = n.indexOf(t, i + t.length)) ranges.push([i, i + t.length]);
      });
      ranges.sort(function (a, b) { return a[0] - b[0]; });
      var out = "", pos = 0;
      ranges.forEach(function (r) {
        if (r[0] < pos) return;
        out += escapeHtml(text.slice(pos, r[0])) + "<mark>" + escapeHtml(text.slice(r[0], r[1])) + "</mark>";
        pos = r[1];
      });
      return out + escapeHtml(text.slice(pos));
    };

    var snippet = function (d, terms) {
      var first = -1;
      terms.forEach(function (t) { var i = d.x.indexOf(t); if (i !== -1 && (first === -1 || i < first)) first = i; });
      if (first === -1) return highlight(d.e.s, terms);
      var start = Math.max(0, first - 40);
      if (start > 0) { var sp = d.e.x.indexOf(" ", start); if (sp !== -1 && sp < first) start = sp + 1; }
      var end = Math.min(d.e.x.length, start + 150);
      return (start > 0 ? "… " : "") + highlight(d.e.x.slice(start, end), terms) + (end < d.e.x.length ? " …" : "");
    };

    var select = function (i) {
      var items = list.children;
      if (!items.length) { sel = -1; return; }
      sel = (i + items.length) % items.length;
      for (var k = 0; k < items.length; k++) items[k].setAttribute("aria-selected", k === sel ? "true" : "false");
      items[sel].scrollIntoView({ block: "nearest" });
      sInput.setAttribute("aria-activedescendant", items[sel].id);
    };

    var run = function () {
      var q = norm(sInput.value.trim());
      var terms = q.split(/\s+/).filter(Boolean);
      list.innerHTML = "";
      sel = -1;
      sInput.removeAttribute("aria-activedescendant");
      if (!terms.length || !docs) {
        status.textContent = "";
        sInput.setAttribute("aria-expanded", "false");
        return;
      }
      results = [];
      docs.forEach(function (d) {
        var score = 0;
        for (var i = 0; i < terms.length; i++) {
          var t = terms[i], s = 0;
          if (d.t.indexOf(t) !== -1) s += d.t.indexOf(t) === 0 ? 14 : 10;
          if (d.g.indexOf(t) !== -1) s += 6;
          if (d.s.indexOf(t) !== -1) s += 3;
          if (d.x.indexOf(t) !== -1) s += 1 + Math.min(4, d.x.split(t).length - 2) * 0.25;
          if (!s) return; // every term must match somewhere
          score += s;
        }
        results.push({ d: d, score: score });
      });
      results.sort(function (a, b) { return b.score - a.score; });
      results = results.slice(0, 20);
      results.forEach(function (r, i) {
        var e = r.d.e;
        var li = document.createElement("li");
        li.id = "sr-" + i;
        li.setAttribute("role", "option");
        var meta = [e.c ? e.c + "/" : "", e.d].concat(e.g.map(function (g) { return "#" + g; })).filter(Boolean).join(" · ");
        li.innerHTML = '<a href="' + escapeHtml(e.u) + '">' +
          '<span class="sr-title">' + highlight(e.t, terms) + (e.l ? ' <span class="badge">' + escapeHtml(e.l) + "</span>" : "") + "</span>" +
          (meta ? '<span class="sr-meta">' + escapeHtml(meta) + "</span>" : "") +
          '<span class="sr-snippet">' + snippet(r.d, terms) + "</span></a>";
        li.addEventListener("mousemove", function () { if (sel !== i) select(i); });
        list.appendChild(li);
      });
      sInput.setAttribute("aria-expanded", results.length ? "true" : "false");
      status.textContent = results.length ? "" : emptyText;
      if (results.length) select(0);
    };

    var open = function () {
      if (dlg.open) return;
      dlg.showModal();
      document.documentElement.classList.add("search-open");
      sInput.select();
      load().then(run);
    };
    dlg.addEventListener("close", function () { document.documentElement.classList.remove("search-open"); });
    dlg.addEventListener("click", function (e) { if (e.target === dlg) dlg.close(); });
    dlg.querySelector(".search-close").addEventListener("click", function () { dlg.close(); });
    sInput.addEventListener("input", function () { if (docs) run(); else load().then(run); });
    sInput.addEventListener("keydown", function (e) {
      if (e.key === "Escape") { e.preventDefault(); dlg.close(); }
      else if (e.key === "ArrowDown") { e.preventDefault(); select(sel + 1); }
      else if (e.key === "ArrowUp") { e.preventDefault(); select(sel - 1); }
      else if (e.key === "Enter" && sel >= 0) {
        e.preventDefault();
        var a = list.children[sel].querySelector("a");
        if (e.metaKey || e.ctrlKey) window.open(a.href, "_blank"); else location.href = a.href;
      }
    });
    document.querySelectorAll(".search-toggle").forEach(function (b) { b.addEventListener("click", open); });
    document.addEventListener("keydown", function (e) {
      var typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName) || document.activeElement.isContentEditable;
      if ((e.key === "k" || e.key === "K") && (e.metaKey || e.ctrlKey)) { e.preventDefault(); open(); }
      else if (e.key === "/" && !typing && !e.metaKey && !e.ctrlKey && !e.altKey) { e.preventDefault(); open(); }
    });
    // Warm the cache when the user shows intent.
    document.querySelectorAll(".search-toggle").forEach(function (b) {
      b.addEventListener("pointerenter", load, { once: true });
      b.addEventListener("focus", load, { once: true });
    });
  } else {
    document.querySelectorAll(".search-toggle").forEach(function (b) { b.hidden = true; });
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
