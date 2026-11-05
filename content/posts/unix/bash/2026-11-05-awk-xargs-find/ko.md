+++
title = "awk, xargs, find: 감이 오기까지 시간이 걸리는 명령어들"
description = "Stack Overflow에서 복사해 쓰지만 제대로 이해하기는 어려운 파이프라인 도구들: awk의 패턴–액션 모델, 공백이 든 파일 이름과 xargs, find -exec의 ;와 +, 그리고 tr, cut, comm, paste, column."
tags = ["bash", "awk", "unix", "shell"]
+++

[sed 글](/ko/posts/unix/bash/sed/)은 "필드를 다루는 일이라면 awk를 쓰라"로 끝났습니다. 이 글은
거기서 이어서, 웬만한 파이프라인에는 꼭 등장하지만 설명은 잘 안 되는 명령들을 다룹니다.
`xargs`, `find -exec`, 그리고 손으로 짰을 반복문을 하나씩 대신해 주는 작은 도구들입니다.

예제는 GNU coreutils/findutils와 `mawk`로 실행했고, 특정 구현에만 있는 기능은 따로 표시했습니다.

## awk: 패턴과 액션

awk 프로그램은 `패턴 { 액션 }` 규칙의 목록입니다. awk는 입력 한 줄(레코드)마다 그 줄을 필드
`$1`, `$2`, … `$NF`로 나누고, 패턴이 일치하는 액션을 실행합니다. 패턴이 없으면 모든 줄에
일치하고, 액션이 없으면 그 줄을 출력합니다.

```bash
awk '{ print $1 }' file                      # 각 줄의 첫 번째 필드
awk -F: '$3 >= 1000 { print $1, $7 }' /etc/passwd
# alice /bin/zsh
# bob /bin/bash
```

`-F`는 입력 필드 구분자를 정합니다(기본값은 공백). 대부분의 일은 몇 개의 내장 변수로 처리됩니다.

| 변수 | 의미 |
|---|---|
| `NR` | 현재 레코드 번호 (모든 파일을 통틀어) |
| `FNR` | 현재 파일 안에서의 레코드 번호 |
| `NF` | 현재 레코드의 필드 수 (`$NF`가 마지막 필드) |
| `FS` / `OFS` | 입력 / 출력 필드 구분자 |
| `BEGIN` / `END` | 첫 줄 전과 마지막 줄 뒤에 실행되는 특수 패턴 |

```bash
awk -F: -v OFS='\t' '{ print $1, $6 }' /etc/passwd   # 사용자<TAB>홈 디렉터리
awk '{ s += $NF } END { print s }' sizes.txt         # 마지막 열 합계
awk -F: '{ printf "%-8s %5d\n", $1, $3 }' /etc/passwd # 열 맞춰 출력
```

### 연관 배열: 한 줄짜리 group by

awk의 배열은 문자열을 키로 쓰는 해시 맵이라서 "group by"가 한 줄로 끝납니다. 일반적인 액세스 로그
형식에서 클라이언트 IP는 `$1`, 경로는 `$7`, 상태 코드는 `$9`, 크기는 `$10`입니다.

```bash
# 203.0.113.7 - - [05/Nov/2026:10:12:01 +0100] "GET /posts/ HTTP/1.1" 200 5120 "-" "curl/8.5"

awk '$9 == 404 { print $7 }' access.log                   # 404를 돌려준 경로
awk '{ bytes[$1] += $10 } END { for (ip in bytes) print bytes[ip], ip }' access.log | sort -rn
```

어디서나 보게 되는 관용구 두 개는 외워 둘 만합니다.

**정렬하지 않고 중복 줄 제거** (처음 나온 줄을 순서대로 유지):

```bash
printf 'b\na\nb\nc\na\n' | awk '!seen[$0]++'
# b
# a
# c
```

`seen[$0]++`는 증가하기 전의 값을 돌려주므로, 어떤 줄이 처음 나오면 0(거짓)입니다. `!`가 그걸
참으로 바꾸고, 액션이 없는 패턴은 그 줄을 출력합니다.

**다른 파일의 키로 걸러내기:**

```bash
awk -F: 'NR == FNR { ok[$1]; next } $1 in ok' allowed-users.txt /etc/passwd
```

첫 번째 파일을 읽는 동안에는 `NR`(전체)과 `FNR`(파일별)이 같으므로, 첫 규칙이 키를 저장하고
`next`로 나머지를 건너뜁니다. 두 번째 파일에서는 더 이상 같지 않으니 `$1 in ok`가 앞에서 본 키를
가진 줄만 남깁니다. 어느 입력도 정렬하지 않는 조인입니다.

## xargs: 줄을 인자로 바꾸기

많은 명령은 파일 이름을 표준 입력이 아니라 인자로 받습니다. 흔히 쓰는 `rm $(find . -name '*.log')`는
이름에 공백이 들어가는 순간 깨지고, 파일이 많으면 명령줄 최대 길이를 넘기도 합니다.

```bash
for f in $(find files -name '*.log'); do echo "[$f]"; done
# [files/a]
# [b.log]        ← "a b.log"가 둘로 쪼개짐
# [files/c.log]
```

`xargs`는 표준 입력에서 항목을 읽어 명령의 인자로 넘기고, 명령줄이 너무 길어지지 않게 나눠서
실행합니다. 어떤 파일 이름에도 안전하게 하려면 양쪽에서 NUL 바이트로 항목을 구분합니다.

```bash
find files -name '*.log' -print0 | xargs -0 rm --
```

| 옵션 | 효과 |
|---|---|
| `-0` | 항목을 NUL로 구분 (`find -print0`, `grep -lZ` 등과 함께) |
| `-n N` | 명령 한 번에 인자 최대 N개 |
| `-P N` | 최대 N개 명령을 병렬 실행 |
| `-I{}` | 입력 한 줄마다 명령 한 번, `{}`를 그 줄로 바꿈 |
| `-r` | GNU: 입력이 비어 있으면 명령을 아예 실행하지 않음 |

```bash
echo 1 2 3 4 | xargs -n 2 echo           # "1 2", "3 4"
printf 'x\ny\n' | xargs -I{} echo "item: {}"
find . -name '*.png' -print0 | xargs -0 -P 4 -n 16 optipng   # 작업자 4개, 한 번에 16개씩
```

:::warning
`-r` 줄은 보기보다 중요합니다. GNU xargs는 입력이 없어도 명령을 한 번 실행하기 때문에, 빈 목록에
`... | xargs rm`을 하면 인자 없는 `rm`이 실행됩니다. `-r`을 붙이거나 `find -exec`를 쓰세요.
:::

## find -exec: `\;`와 `+`

`find`는 명령을 직접 실행할 수 있어서 따옴표 문제를 아예 피할 수 있습니다.

```bash
find files -name '*.log' -exec echo {} \;    # 파일마다 echo 한 번
find files -name '*.log' -exec echo {} +     # 모든 파일을 넣어 echo 한 번, xargs처럼
```

`\;`는 파일마다 프로세스를 하나씩 띄우고, `+`는 한 번 호출에 들어갈 만큼 파일을 묶어서 넘기므로
파일이 많을 때 훨씬 빠릅니다. 명령이 파일을 딱 하나만 받을 때만 `\;`를 쓰세요.

조건은 암묵적인 AND로 이어지고, OR은 `-o`, 묶을 때는 이스케이프한 괄호를 씁니다. `-prune`은
디렉터리 하나를 통째로 건너뜁니다.

```bash
find files \( -name '*.log' -o -name '*.txt' \) -type f
find proj -path proj/node_modules -prune -o -name '*.js' -print
find /var/log -type f -mtime -1 -size +100M          # 하루 안에 수정됐고 100MiB보다 큰 파일
find . -name '*.tmp' -delete                         # find 자체 삭제 기능, xargs 불필요
```

`-prune -o … -print` 형태는 이렇게 읽으면 이해됩니다. "경로가 `node_modules`면 가지치기하고(아무것도
출력하지 않고), 아니면 `.js` 파일일 때 출력한다." 마지막의 `-print`를 빼면 가지치기한 디렉터리
자신도 출력됩니다.

## 반복문을 대신하는 작은 도구들

하나같이 한 가지 일만 하고, 하나같이 `while read` 반복문을 짤 일을 자주 덜어 줍니다.

`tr`은 문자를 바꾸거나 지웁니다.

```bash
echo 'Hello   World' | tr -s ' ' | tr 'a-z' 'A-Z'   # HELLO WORLD
tr -d '\r' < windows.txt > unix.txt                 # CRLF 줄 끝 제거
```

`cut`은 구분자나 글자 위치로 열을 골라냅니다.

```bash
cut -d: -f1,7 /etc/passwd     # 사용자:셸
cut -c1-8 file                # 각 줄의 앞 8글자
```

`comm`은 정렬된 두 파일을 줄 단위로 비교해서 세 열로 보여 줍니다. 첫 번째 파일에만 있는 줄, 두
번째 파일에만 있는 줄, 양쪽에 다 있는 줄입니다. `-1`, `-2`, `-3`으로 열을 숨깁니다. 프로세스 치환
`<(…)`과 함께 쓰면 임시 파일이 필요 없습니다. 입력이 정렬되어 있어야 한다는 점만 잊지 마세요.

```bash
comm -12 <(sort a.txt) <(sort b.txt)     # 양쪽에 다 있는 줄
comm -23 <(sort a.txt) <(sort b.txt)     # a.txt에만 있는 줄
```

`paste`는 줄을 합칩니다. 나란히 붙이거나, 한 파일의 모든 줄을 한 줄로 만듭니다.

```bash
paste names.txt sizes.txt     # 이름<TAB>크기
paste -sd, list.txt           # a,b,c
```

`sort -k`, `uniq -c`, `column -t`로 대부분의 보고서가 마무리됩니다.

```bash
sort -t, -k2,2n data.csv      # 쉼표로 나눈 2번째 필드 기준 숫자 정렬
sort | uniq -c | sort -rn     # 등장 횟수 세기, 많은 순
column -t report.txt          # 공백으로 나뉜 열 맞추기
```

`-k2,2n`은 "키가 2번째 필드에서 시작해서 2번째 필드에서 끝나고, 숫자로 비교"라는 뜻입니다. 그냥
`-k2`라고 쓰면 2번째 필드부터 줄 끝까지를 비교합니다.

`tee`는 입력을 파일과 표준 출력에 동시에 씁니다. 파이프라인에서 root 소유 파일에 쓸 때의 표준적인
방법이기도 합니다. `sudo cmd > file`에서 리다이렉션은 `sudo`가 아니라 지금 셸이 처리하기 때문입니다.

```bash
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.d/99-local.conf > /dev/null
```

## 한데 모으기

```bash
# 요청이 가장 많은 클라이언트 IP 10개
awk '{ print $1 }' access.log | sort | uniq -c | sort -rn | head

# 하루 안에 바뀌었고 아직 TODO가 남아 있는 소스 파일
find src -type f -mtime -1 -print0 | xargs -0 grep -l TODO

# /etc/passwd에는 있지만 허용 목록에는 없는 사용자
comm -23 <(cut -d: -f1 /etc/passwd | sort) <(sort allowed-users.txt)
```
