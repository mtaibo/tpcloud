#!/usr/bin/env bash
# TPCloud installer: a step-by-step setup in the terminal, then everything installed and started.
#
#   curl -fsSL https://raw.githubusercontent.com/mtaibo/tpcloud/main/install.sh | bash
#
# Run it again to update, or to change modules or how TPCloud is reached.
# Options: --dry-run (ask everything, change nothing, print what it would do),
#          --update  (no questions: get the latest version of everything and restart it).

set -Eeuo pipefail

REPO_BASE="${TPCLOUD_REPO_BASE:-https://github.com/mtaibo}"
BRANCH="${TPCLOUD_BRANCH:-main}"
LOG="${TMPDIR:-/tmp}/tpcloud-install.log"
DRY_RUN=0
ONLY_UPDATE=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --update) ONLY_UPDATE=1 ;;
    *) echo "Unknown option: $arg (use --dry-run or --update)" >&2; exit 1 ;;
  esac
done

# tpcloud holds everything but Home; tphome, when it's installed, is a clone next to it. Both sit
# next to this script when it's run from a clone, otherwise in TPCLOUD_BASE or the home folder.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" 2>/dev/null && pwd || true)"
if [[ -n "$script_dir" && -f "$script_dir/docker-compose.yml" && -d "$script_dir/services/gateway" ]]; then
  BASE="$(dirname "$script_dir")"
else
  BASE="${TPCLOUD_BASE:-$HOME}"
fi
CORE="$BASE/tpcloud"

# ── Terminal ────────────────────────────────────────────────────────────────────────────────────

# Piped from curl, stdin is the script itself: answers are read from the terminal instead.
# An update asks nothing, so it also runs without one (from cron or over ssh without -t).
if ! { exec 3</dev/tty; } 2>/dev/null; then
  (( ONLY_UPDATE )) || { echo "This installer needs a terminal to ask its questions." >&2; exit 1; }
  exec 3</dev/null
fi

if [[ -t 1 ]] && [[ "$(tput colors 2>/dev/null || echo 0)" -ge 8 ]]; then
  B=$'\e[1m'; D=$'\e[2m'; R=$'\e[0m'; ACC=$'\e[38;5;75m'; OK=$'\e[38;5;114m'; WARN=$'\e[38;5;215m'; ERR=$'\e[38;5;203m'
else
  B=""; D=""; R=""; ACC=""; OK=""; WARN=""; ERR=""
fi

cols() { tput cols 2>/dev/null || echo 80; }

center() {
  local text="$1" plain width pad
  plain="$(printf '%s' "$text" | sed $'s/\e\\[[0-9;]*m//g')"
  width="$(cols)"
  pad=$(( (width - ${#plain}) / 2 ))
  (( pad < 0 )) && pad=0
  printf '%*s%s\n' "$pad" '' "$text"
}

STEP=0
STEPS=6

header() {
  printf '\e[H\e[2J'
  echo
  center "${ACC}${B}╭───────────────────────────╮${R}"
  center "${ACC}${B}│      T P C l o u d        │${R}"
  center "${ACC}${B}╰───────────────────────────╯${R}"
  (( STEP > 0 )) && center "${D}Step ${STEP} of ${STEPS}${R}"
  echo
}

say() { printf '   %s\n' "$*"; }
gap() { echo; }

key() {
  local k
  IFS= read -rsn1 k <&3 || k=""
  printf '%s' "$k"
}

common_choices() {
  gap
  say "${ACC}(r)${R}  Restart from the beginning."
  say "${ACC}(q)${R}  Quit and do nothing."
  gap
}

# choose <title> <option>... → sets CHOICE to the number picked.
choose() {
  local title="$1"; shift
  local options=("$@") i k valid
  while true; do
    header
    say "${B}${title}${R}"
    gap
    for i in "${!options[@]}"; do
      say "${ACC}($((i + 1)))${R}  ${options[$i]}"
      gap
    done
    common_choices
    valid="$(seq -s '' 1 ${#options[@]})rq"
    printf '   Choice [%s]: ' "$valid"
    k="$(key)"; echo
    case "$k" in
      q) quit ;;
      r) restart; return ;;
      [1-9]) if (( k <= ${#options[@]} )); then CHOICE="$k"; return; fi ;;
    esac
  done
}

# ask <prompt> <default> → sets ANSWER.
ask() {
  local prompt="$1" default="${2:-}" reply
  if [[ -n "$default" ]]; then
    printf '   %s %s[%s]%s: ' "$prompt" "$D" "$default" "$R"
  else
    printf '   %s: ' "$prompt"
  fi
  IFS= read -r reply <&3 || reply=""
  ANSWER="${reply:-$default}"
}

ask_secret() {
  local prompt="$1" reply
  printf '   %s: ' "$prompt"
  IFS= read -rs reply <&3 || reply=""
  echo
  ANSWER="$reply"
}

pause() {
  gap
  printf '   %sPress any key to continue, or q to quit.%s' "$D" "$R"
  [[ "$(key)" == "q" ]] && quit
  echo
}

quit() {
  printf '\n\n   Nothing was changed.\n\n'
  exit 0
}

RESTART=0
restart() { RESTART=1; return 0; }

fail() {
  printf '\n   %s✗ %s%s\n' "$ERR" "$*" "$R" >&2
  if [[ -s "$LOG" ]]; then
    printf '\n   %sLast lines of %s:%s\n\n' "$D" "$LOG" "$R" >&2
    tail -n 20 "$LOG" | sed 's/^/     /' >&2
  fi
  echo >&2
  exit 1
}

trap 'fail "Something went wrong (line $LINENO)."' ERR

# ── Running things ──────────────────────────────────────────────────────────────────────────────

SUDO=""
(( EUID != 0 )) && SUDO="sudo"

DOCKER=(docker)

# run <description> <command…>: one line per step, with the command's output kept in the log.
run() {
  local desc="$1"; shift
  if (( DRY_RUN )); then
    printf '   %s•%s %s\n     %s%s%s\n' "$ACC" "$R" "$desc" "$D" "$*" "$R"
    return
  fi
  printf '   %s…%s %s' "$D" "$R" "$desc"
  local spin='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏' i=0
  ( trap - ERR; "$@" ) >>"$LOG" 2>&1 &
  local pid=$!
  while kill -0 "$pid" 2>/dev/null; do
    printf '\r   %s%s%s %s' "$ACC" "${spin:i++%${#spin}:1}" "$R" "$desc"
    sleep 0.1
  done
  if wait "$pid"; then
    printf '\r   %s✓%s %s\n' "$OK" "$R" "$desc"
  else
    printf '\r   %s✗%s %s\n' "$ERR" "$R" "$desc"
    fail "$desc failed."
  fi
}

# Steps that talk to the person (sudo's password, Tailscale's sign-in link) run in the open.
run_open() {
  local desc="$1"; shift
  if (( DRY_RUN )); then
    printf '   %s•%s %s\n     %s%s%s\n' "$ACC" "$R" "$desc" "$D" "$*" "$R"
    return
  fi
  printf '   %s›%s %s\n' "$ACC" "$R" "$desc"
  "$@" <&3 || fail "$desc failed."
}

compose() {
  local dir="$1"; shift
  (cd "$dir" && "${DOCKER[@]}" compose "$@")
}

# env_set <file> <key> <value>: replaces the key's line, or adds it, leaving every other line alone.
env_set() {
  local file="$1" key="$2" value="$3"
  if (( DRY_RUN )); then
    printf '     %s%s: %s=%s%s\n' "$D" "${file#"$BASE"/}" "$key" "$( [[ "$key" == *TOKEN* ]] && echo '•••' || echo "$value")" "$R"
    return
  fi
  touch "$file"
  chmod 600 "$file"
  local tmp
  tmp="$(mktemp)"
  grep -v "^${key}=" "$file" > "$tmp" || true
  printf '%s=%s\n' "$key" "$value" >> "$tmp"
  cat "$tmp" > "$file"
  rm -f "$tmp"
}

env_get() {
  local file="$1" key="$2"
  [[ -f "$file" ]] || return 0
  grep -E "^${key}=" "$file" | tail -n 1 | cut -d= -f2- | sed 's/[[:space:]]*#.*$//; s/[[:space:]]*$//' || true
}

lan_ip() {
  if command -v hostname >/dev/null && hostname -I >/dev/null 2>&1; then
    hostname -I | awk '{print $1}'
  else
    ipconfig getifaddr en0 2>/dev/null || echo "127.0.0.1"
  fi
}

# ── Questions ───────────────────────────────────────────────────────────────────────────────────

WANT_STORE=1
WANT_HOME=0
STORAGE=""
ACCESS=""
DOMAIN=""
TUNNEL_TOKEN=""
OWNER_EMAIL=""
OWNER_PASSWORD=""
FRESH=1

ask_existing() {
  [[ -f "$CORE/.env" ]] || return 0
  FRESH=0
  STEP=0
  CHOICE=""
  while [[ -z "$CHOICE" ]]; do
    RESTART=0
    choose "TPCloud is already installed in ${CORE}. What do you want to do?" \
      "${B}Update${R}         Get the latest version of every module and restart them." \
      "${B}Change setup${R}   Pick modules or how TPCloud is reached again. Your files and accounts stay."
  done
  if [[ "$CHOICE" == 1 ]]; then
    MODE=update
  else
    MODE=configure
  fi
}

ask_welcome() {
  STEP=0
  header
  say "${B}This sets up your own TPCloud on this machine.${R}"
  gap
  say "It will ask which modules you want and how you'll reach them, then install"
  say "Docker if it's missing and start everything. Nothing leaves this machine:"
  say "your files and accounts stay here, on storage you choose."
  gap
  say "${D}Installing into ${BASE}/tpcloud (and ${BASE}/tphome for Home).${R}"
  pause
}

ask_modules() {
  STEP=1
  local k
  while true; do
    header
    say "${B}Which modules do you want?${R}"
    gap
    say "${D}Sign-in and accounts are always included.${R}"
    gap
    say "${ACC}(1)${R}  $( ((WANT_STORE)) && echo "${OK}[x]${R}" || echo "[ ]")  ${B}Store${R}   Your files, photo backup from the app and share links."
    gap
    say "${ACC}(2)${R}  $( ((WANT_HOME)) && echo "${OK}[x]${R}" || echo "[ ]")  ${B}Home${R}    Blinds and lights, with tphome's own Wi-Fi switches."
    gap
    say "${ACC}(enter)${R}  Continue."
    common_choices
    printf '   Choice [12rq or enter]: '
    k="$(key)"; echo
    case "$k" in
      1) WANT_STORE=$(( 1 - WANT_STORE )) ;;
      2) WANT_HOME=$(( 1 - WANT_HOME )) ;;
      "") return ;;
      q) quit ;;
      r) restart; return ;;
    esac
  done
}

ask_storage() {
  STEP=2
  (( WANT_STORE )) || return 0
  local current suggested
  current="$(env_get "$CORE/.env" EXTERNAL_DISK_PATH)"
  if [[ -n "$current" ]]; then
    suggested="$current"
  elif [[ -d /mnt/external ]]; then
    suggested="/mnt/external"
  else
    suggested="$BASE/tpcloud-storage"
  fi
  header
  say "${B}Where should Store keep your files?${R}"
  gap
  say "A folder on a big disk is best (an external drive mounted somewhere like"
  say "/mnt/external). Each person gets their own folder inside, plus a shared one."
  gap
  ask "Folder" "$suggested"
  STORAGE="${ANSWER%/}"
}

ask_access() {
  STEP=3
  local host
  host="$(hostname -s 2>/dev/null || echo tpcloud)"
  choose "How will you reach TPCloud?" \
    "${B}Only at home${R}
        On this network, at https://${host}.local. Nothing is reachable from
        outside; each device has to trust this machine's certificate once." \
    "${B}Anywhere, privately${R}  ${OK}Recommended${R}
        With Tailscale, a free VPN: only your own devices reach it, from
        anywhere, with a real certificate and no ports opened." \
    "${B}Your domain, through Cloudflare${R}
        Public at e.g. https://cloud.example.com through a Cloudflare Tunnel.
        No ports opened on your router." \
    "${B}Your domain, with an open port${R}
        Public at your domain, with port 443 forwarded to this machine on your
        router. The certificate is obtained automatically."
  (( RESTART )) && return
  case "$CHOICE" in
    1) ACCESS=local ;;
    2) ACCESS=tailscale ;;
    3) ACCESS=tunnel ;;
    4) ACCESS=domain ;;
  esac

  if [[ "$ACCESS" == tunnel || "$ACCESS" == domain ]]; then
    local current
    current="$(env_get "$CORE/.env" TPCLOUD_URL)"
    current="${current#https://}"
    header
    say "${B}Which address?${R}"
    gap
    say "The full name people will type, e.g. cloud.example.com. Everything lives under"
    say "it: /store, /home, /login…"
    gap
    while true; do
      ask "Address" "$current"
      DOMAIN="${ANSWER#https://}"; DOMAIN="${DOMAIN%%/*}"
      [[ "$DOMAIN" =~ ^[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]] && break
      say "${WARN}That doesn't look like an address (e.g. cloud.example.com).${R}"
    done
  fi

  if [[ "$ACCESS" == tunnel ]]; then
    header
    say "${B}Your Cloudflare Tunnel${R}"
    gap
    say "1. In Cloudflare, open Zero Trust → Networks → Tunnels → Create a tunnel"
    say "   (type Cloudflared) and copy its token."
    say "2. In the tunnel's Public Hostnames, add ${B}${DOMAIN}${R} pointing to"
    say "   ${B}HTTP  gateway:80${R}."
    gap
    local existing
    existing="$(env_get "$CORE/.env" TUNNEL_TOKEN)"
    while true; do
      if [[ -n "$existing" ]]; then
        ask_secret "Tunnel token (enter to keep the current one)"
        TUNNEL_TOKEN="${ANSWER:-$existing}"
      else
        ask_secret "Tunnel token"
        TUNNEL_TOKEN="$ANSWER"
      fi
      [[ -n "$TUNNEL_TOKEN" ]] && break
    done
  fi

  if [[ "$ACCESS" == domain ]]; then
    header
    say "${B}Before going on${R}"
    gap
    say "1. Point ${B}${DOMAIN}${R} (an A record) at your home's public IP."
    say "2. On your router, forward TCP port ${B}443${R} to ${B}$(lan_ip)${R}."
    gap
    say "${WARN}TPCloud will be open to the internet. Turn on two-factor authentication${R}"
    say "${WARN}for every account as soon as you sign in.${R}"
    pause
  fi
}

ask_owner() {
  STEP=4
  if [[ -f "$CORE/data/auth/tpauth.db" ]]; then
    OWNER_EMAIL=""
    return 0
  fi
  header
  say "${B}Your account${R}"
  gap
  say "The first account, and the server's admin. You can add more people later"
  say "from Accounts."
  gap
  while true; do
    ask "Email" ""
    OWNER_EMAIL="$(printf '%s' "$ANSWER" | tr '[:upper:]' '[:lower:]')"
    [[ "$OWNER_EMAIL" =~ ^[^@[:space:]]+@[^@[:space:]]+\.[^@[:space:]]+$ ]] && break
    say "${WARN}That doesn't look like an email.${R}"
  done
  while true; do
    ask_secret "Password (12 characters or more)"
    OWNER_PASSWORD="$ANSWER"
    if (( ${#OWNER_PASSWORD} < 12 )); then
      say "${WARN}Too short.${R}"; continue
    fi
    ask_secret "Same password again"
    [[ "$ANSWER" == "$OWNER_PASSWORD" ]] && break
    say "${WARN}They don't match.${R}"
  done
}

summary() {
  STEP=5
  local modules="Sign-in, Accounts" where
  (( WANT_STORE )) && modules+=", Store"
  (( WANT_HOME )) && modules+=", Home"
  case "$ACCESS" in
    local) where="Only at home, https://$(hostname -s 2>/dev/null || echo tpcloud).local" ;;
    tailscale) where="Anywhere, privately, through Tailscale" ;;
    tunnel) where="https://${DOMAIN}, through a Cloudflare Tunnel" ;;
    domain) where="https://${DOMAIN}, port 443 open" ;;
  esac
  while true; do
    header
    say "${B}Ready to install${R}"
    gap
    say "Modules     ${modules}"
    (( WANT_STORE )) && say "Files in    ${STORAGE}"
    say "Reached     ${where}"
    [[ -n "$OWNER_EMAIL" ]] && say "Admin       ${OWNER_EMAIL}"
    say "Folder      ${BASE}"
    gap
    say "${ACC}(y)${R}  Install."
    common_choices
    printf '   Choice [yrq]: '
    case "$(key)" in
      y) echo; return ;;
      q) quit ;;
      r) restart; return ;;
    esac
  done
}

# ── Installing ──────────────────────────────────────────────────────────────────────────────────

ensure_tools() {
  if [[ "$(uname -s)" != Linux ]] && (( ! DRY_RUN )); then
    fail "TPCloud's server runs on Linux (a Raspberry Pi, an old PC, a VPS…)."
  fi
  (( DRY_RUN )) && return 0

  [[ -n "$SUDO" ]] && { run_open "Checking you can use sudo" sudo -v; }

  local missing=()
  command -v git >/dev/null || missing+=(git)
  command -v curl >/dev/null || missing+=(curl)
  command -v qrencode >/dev/null || missing+=(qrencode)
  [[ "$ACCESS" == local ]] && ! command -v avahi-daemon >/dev/null && missing+=(avahi-daemon)
  if (( ${#missing[@]} )); then
    command -v apt-get >/dev/null || fail "Please install: ${missing[*]}"
    run "Installing ${missing[*]}" $SUDO apt-get install -y "${missing[@]}"
  fi

  if ! command -v docker >/dev/null; then
    run "Installing Docker" sh -c "curl -fsSL https://get.docker.com | $SUDO sh"
    [[ -n "$SUDO" ]] && run "Letting $(id -un) use Docker" $SUDO usermod -aG docker "$(id -un)"
  fi
  # Without the docker group (or right after joining it, before signing in again), through sudo,
  # keeping the two variables the account step hands to the container.
  if ! docker info >/dev/null 2>&1 && [[ -n "$SUDO" ]]; then
    DOCKER=(sudo "--preserve-env=OWNER_EMAIL,OWNER_PASSWORD" docker)
  fi
}

fetch_repo() {
  local name="$1" dir="$BASE/$1"
  if [[ -d "$dir/.git" ]]; then
    run "Updating ${name}" git -C "$dir" pull --ff-only origin "$BRANCH"
  else
    run "Downloading ${name}" git clone --branch "$BRANCH" "$REPO_BASE/$name.git" "$dir"
  fi
}

setup_tailscale() {
  if ! command -v tailscale >/dev/null; then
    run "Installing Tailscale" sh -c "curl -fsSL https://tailscale.com/install.sh | $SUDO sh"
  fi
  if (( ! DRY_RUN )) && ! tailscale status >/dev/null 2>&1; then
    gap
    say "Sign in to Tailscale with the link below (the same account as on your phone)."
    gap
    run_open "Connecting this machine to Tailscale" $SUDO tailscale up
  fi
  if (( DRY_RUN )); then
    DOMAIN="<this machine>.<your tailnet>.ts.net"
  else
    DOMAIN="$(tailscale status --json | python3 -c 'import json,sys; print(json.load(sys.stdin)["Self"]["DNSName"].rstrip("."))' 2>/dev/null \
      || tailscale status --json | sed -n 's/.*"DNSName": *"\([^"]*\)".*/\1/p' | head -n 1 | sed 's/\.$//')"
    [[ -n "$DOMAIN" ]] || fail "Couldn't find this machine's Tailscale name."
  fi
  # HTTPS certificates must be on for the tailnet; `tailscale serve` says how if they aren't.
  run_open "Serving TPCloud at https://${DOMAIN} on your tailnet" $SUDO tailscale serve --bg --https=443 http://127.0.0.1:8080
}

configure() {
  local env="$CORE/.env" modules=() url site tls="none" https_publish="" ota_publish="" profiles=() host
  host="$(hostname -s 2>/dev/null || echo tpcloud)"
  (( WANT_STORE )) && modules+=(store)
  (( WANT_HOME )) && modules+=(home)

  case "$ACCESS" in
    local) url="https://${host}.local"; site="https://${host}.local"; tls="internal"; https_publish="443" ;;
    tailscale) setup_tailscale; url="https://${DOMAIN}"; site=":80" ;;
    tunnel) url="https://${DOMAIN}"; site=":80"; profiles+=(tunnel) ;;
    domain) url="https://${DOMAIN}"; site="${DOMAIN}"; https_publish="443" ;;
  esac
  (( WANT_HOME )) && ota_publish="80"
  (( WANT_STORE )) && profiles+=(store)

  [[ -f "$env" ]] || { (( DRY_RUN )) || printf '# Written by install.sh. Run it again to change these.\n' > "$env"; }
  [[ -n "$(env_get "$env" DATABASE_URL)" ]] || env_set "$env" DATABASE_URL "sqlite:///data/tpauth.db"
  env_set "$env" TPCLOUD_URL "$url"
  env_set "$env" TPCLOUD_SITE "$site"
  env_set "$env" TPCLOUD_TLS "$tls"
  env_set "$env" TPCLOUD_MODULES "$(IFS=,; echo "${modules[*]}")"
  env_set "$env" TPCLOUD_HTTPS_PUBLISH "$https_publish"
  env_set "$env" TPCLOUD_OTA_PUBLISH "$ota_publish"
  env_set "$env" COMPOSE_PROFILES "$(IFS=,; echo "${profiles[*]}")"
  [[ "$ACCESS" == tunnel ]] && env_set "$env" TUNNEL_TOKEN "$TUNNEL_TOKEN"
  TPCLOUD_URL_FINAL="$url"

  if (( WANT_STORE )); then
    run "Creating ${STORAGE}" $SUDO mkdir -p "$STORAGE"
    env_set "$env" EXTERNAL_DISK_PATH "$STORAGE"
    env_set "$env" AUTH_SERVICE_URL "http://auth:8000"
  fi
  if (( WANT_HOME )); then
    local henv="$BASE/tphome/.env"
    if [[ ! -f "$henv" ]] && (( ! DRY_RUN )); then
      cp "$BASE/tphome/.env.example" "$henv"
      chmod 600 "$henv"
    fi
    env_set "$henv" MQTT_IP "$(lan_ip)"
  fi
}

start() {
  run "Starting TPCloud (the first time builds everything, which takes a while)" compose "$CORE" up -d --build --remove-orphans
  if [[ -n "$OWNER_EMAIL" ]]; then
    if (( DRY_RUN )); then
      run "Creating your account" "${DOCKER[@]}" compose run --rm auth python scripts/bootstrap.py
    else
      # Passed through the environment, never on a command line or in a file.
      export OWNER_EMAIL OWNER_PASSWORD
      run "Creating your account" sh -c "cd '$CORE' && ${DOCKER[*]} compose run --rm -e OWNER_EMAIL -e OWNER_PASSWORD auth python scripts/bootstrap.py"
      unset OWNER_PASSWORD
    fi
  fi
  # Store is part of tpcloud, behind the `store` profile: left out this time, it's stopped and its
  # data kept. (Naming its services turns the profile on just for this command.)
  if (( ! WANT_STORE )); then
    run "Stopping Store" compose "$CORE" rm --stop --force store-api store-frontend share-frontend photos-frontend
  fi
  if (( WANT_HOME )); then
    run "Starting tphome" compose "$BASE/tphome" up -d --build --remove-orphans
  elif [[ -f "$BASE/tphome/docker-compose.yml" ]]; then
    run "Stopping tphome" compose "$BASE/tphome" down
  fi
  return 0
}

update() {
  local name
  STEP=0
  header
  say "${B}Updating${R}"
  gap
  ensure_tools
  for name in tpcloud tphome; do
    [[ -d "$BASE/$name/.git" ]] || continue
    fetch_repo "$name"
  done
  run "Restarting TPCloud" compose "$CORE" up -d --build --remove-orphans
  if [[ -f "$BASE/tphome/.env" ]]; then
    run "Restarting tphome" compose "$BASE/tphome" up -d --build --remove-orphans
  fi
  gap
  say "${OK}${B}Up to date.${R}"
  gap
}

finish() {
  local url="$TPCLOUD_URL_FINAL"
  STEP=0
  echo
  say "${OK}${B}TPCloud is running.${R}"
  gap
  say "Open ${B}${url}${R} in a browser, or type it into the TPCloud app."
  gap
  if command -v qrencode >/dev/null && (( ! DRY_RUN )); then
    qrencode -t ansiutf8 -m 2 "$url" | sed 's/^/   /'
    gap
  fi
  say "Next:"
  say "  • Sign in and turn on two-factor authentication in Accounts."
  case "$ACCESS" in
    local)
      say "  • Each device has to trust this machine's certificate once. It's saved at"
      say "    ${B}${CORE}/tpcloud-local-ca.crt${R}: open it on the device and trust it."
      if (( ! DRY_RUN )); then
        sleep 2
        compose "$CORE" exec -T gateway cat /data/caddy/pki/authorities/local/root.crt > "$CORE/tpcloud-local-ca.crt" 2>>"$LOG" || true
      fi
      ;;
    tailscale)
      say "  • Install Tailscale on your phone and computers, signed in to the same account." ;;
    tunnel)
      say "  • If it doesn't load, check the tunnel's public hostname points to HTTP gateway:80." ;;
  esac
  (( WANT_HOME )) && say "  • Home's switches find this machine at $(lan_ip) (MQTT on port 1883)."
  say "  • Run this installer again to update, or to change modules."
  gap
}

# ── Main ────────────────────────────────────────────────────────────────────────────────────────

main() {
  : > "$LOG" 2>/dev/null || LOG="/dev/null"
  MODE=install
  if (( ONLY_UPDATE )); then
    [[ -f "$CORE/.env" ]] || fail "TPCloud isn't installed in ${CORE} yet: run this without --update first."
    update
    exit 0
  fi
  ask_existing
  if [[ "$MODE" == update ]]; then
    update
    exit 0
  fi

  if [[ -f "$CORE/.env" ]]; then
    local modules
    modules="$(env_get "$CORE/.env" TPCLOUD_MODULES)"
    [[ ",$modules," == *",store,"* ]] && WANT_STORE=1 || WANT_STORE=0
    [[ ",$modules," == *",home,"* ]] && WANT_HOME=1 || WANT_HOME=0
  fi

  while true; do
    RESTART=0
    (( FRESH )) && ask_welcome
    ask_modules;  (( RESTART )) && continue
    ask_storage;  (( RESTART )) && continue
    ask_access;   (( RESTART )) && continue
    ask_owner;    (( RESTART )) && continue
    summary;      (( RESTART )) && continue
    break
  done

  STEP=6
  header
  say "${B}Installing${R}"
  (( DRY_RUN )) && say "${WARN}Dry run: showing what would happen, changing nothing.${R}"
  gap
  ensure_tools
  fetch_repo tpcloud
  (( WANT_HOME )) && fetch_repo tphome
  configure
  start
  finish
}

main "$@"
