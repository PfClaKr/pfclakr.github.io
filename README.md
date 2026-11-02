# pfclakr.github.io

Rust로 직접 만든 다국어(🇬🇧 English · 🇫🇷 Français · 🇰🇷 한국어) 정적 기술 블로그.

- **빌드 타임 렌더링**: `pulldown-cmark`로 마크다운 → HTML, 결과물은 순수 정적 HTML (SEO 친화)
- **코드 하이라이팅**: `syntect` + `two-face` (TOML, TS, Dockerfile 등 포함), 라이트/다크 테마 자동
- **Mermaid** 다이어그램: ```` ```mermaid ```` 블록이 있는 페이지에서만 mermaid.js 로드
- **GFM**: 표, 각주, 체크리스트, 취소선, `> [!NOTE]` 알림 블록, 헤딩 앵커 + 목차(TOC)
- **댓글**: [giscus](https://giscus.app) (GitHub Discussions) — 번역본끼리 같은 스레드 공유
- **SEO**: canonical, `hreflang` + `x-default`, Open Graph / Twitter Card, JSON-LD (`BlogPosting`, `BreadcrumbList`, `Person`), 다국어 `sitemap.xml`, `robots.txt`, 언어별 Atom 피드
- **다크 모드**, 모바일 대응, 코드 복사 버튼, 이전/다음 글 내비게이션

## 빠른 시작

```bash
cargo run --release -- serve        # http://127.0.0.1:1111 (저장하면 자동 새로고침)
cargo run --release -- build        # public/ 에 사이트 생성
cargo run --release -- new my-post ko   # 새 글 초안 생성
cargo test
```

`serve`는 `content/`, `templates/`, `static/`, `i18n/`, `config.toml` 변경을 감지해서 다시 빌드합니다.
(Rust 코드를 바꿨을 때는 서버를 재시작하세요.)

## 디렉터리 구조

```text
config.toml               사이트 설정 (URL, 언어, 프로필, SEO, 댓글)
i18n/{en,fr,ko}.toml      UI 문자열, 사이트 제목/설명, 날짜 형식
content/
  home/{lang}.md          홈(명함) 자기소개
  pages/about/{lang}.md   고정 페이지 → /about/, /fr/about/, /ko/about/
  posts/
    2026-11-01-hello-world/
      en.md  fr.md  ko.md     같은 폴더 = 서로의 번역본
      diagram.png             글에 쓰는 이미지 (상대 경로로 참조)
templates/                Tera 템플릿 (HTML 구조)
static/                   CSS, JS, 이미지, 파비콘 (그대로 복사됨)
src/                      사이트 생성기 (Rust)
  main.rs                 CLI (build / serve / new)
  content.rs              콘텐츠 로딩, front matter
  markdown.rs             마크다운 렌더링, 하이라이팅, mermaid, TOC
  site.rs                 URL, SEO 메타데이터, 사이트맵, 피드, 출력
  serve.rs                개발 서버 + 라이브 리로드
```

## 글 쓰기

`content/posts/<YYYY-MM-DD>-<slug>/<lang>.md`:

```markdown
+++
title = "Rust 소유권 이해하기"
description = "검색 결과와 링크 미리보기에 쓰이는 요약 (생략 시 첫 문단)"
date = 2026-11-02          # 생략 시 폴더명의 날짜 사용
updated = 2026-11-10       # 선택
tags = ["rust", "memory"]
draft = false              # true면 `--drafts` 옵션 없이는 빌드되지 않음
image = "/img/ownership.png"  # 선택: 글 전용 OG 이미지 (1200×630)
slug = "rust-ownership"    # 선택: 이 언어의 URL slug
toc = true                 # 선택: 목차 표시
comments = true            # 선택: 댓글 표시
+++

본문...
```

URL 규칙 (기본 언어 = `en`, 루트에 배치):

| 페이지 | en | fr | ko |
|---|---|---|---|
| 홈 | `/` | `/fr/` | `/ko/` |
| 글 목록 | `/posts/` | `/fr/posts/` | `/ko/posts/` |
| 글 | `/posts/<slug>/` | `/fr/posts/<slug>/` | `/ko/posts/<slug>/` |
| 태그 | `/tags/<tag>/` | `/fr/tags/<tag>/` | `/ko/tags/<tag>/` |

번역되지 않은 글은 `show_untranslated = true`일 때 다른 언어 목록에도 **원문 언어 배지**와 함께 표시됩니다.

## 배포 (GitHub Pages)

`main`에 push하면 `.github/workflows/deploy.yml`이 테스트 → 빌드 → 배포합니다.

> [!IMPORTANT]
> 최초 1회: 저장소 **Settings → Pages → Build and deployment → Source**를 **GitHub Actions**로 바꿔주세요.
> (예전 워크플로는 `gh-pages` 브랜치를 썼지만, 이제는 공식 Pages 액션으로 배포합니다. `gh-pages` 브랜치는 지워도 됩니다.)

## 댓글 활성화 (giscus)

1. 저장소 **Settings → General → Features → Discussions** 체크
2. Discussions에 `Comments` 카테고리 생성 (형식: *Announcement* 추천 — 스팸 스레드 방지)
3. [giscus 앱](https://github.com/apps/giscus) 설치
4. [giscus.app](https://giscus.app)에서 저장소 입력 → 나오는 `data-repo-id`, `data-category-id`를 `config.toml`의 `[comments]`에 입력

`repo_id`가 비어 있으면 댓글 영역은 숨겨집니다.

## SEO 체크리스트

- [ ] `config.toml`의 `[author]`(이름, LinkedIn, 이메일)와 `i18n/*.toml`의 `site_title`, `tagline` 수정
- [ ] `static/img/avatar.svg`를 실제 사진으로 교체 (`config.toml`의 `avatar` 경로도)
- [ ] [Google Search Console](https://search.google.com/search-console)에 사이트 등록 → `google_site_verification` 입력 → `https://pfclakr.github.io/sitemap.xml` 제출
- [ ] [네이버 서치어드바이저](https://searchadvisor.naver.com)에 등록 → `naver_site_verification` 입력 → 사이트맵/RSS 제출
- [ ] 글마다 `description`을 직접 쓰기 (120–160자 권장)
