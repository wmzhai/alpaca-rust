#!/usr/bin/env bash
set -euo pipefail

readonly SERVICE_LABEL="com.alpaca-rust.alpaca-mock"
readonly SYSTEMD_UNIT_NAME="alpaca-mock.service"
readonly DEFAULT_LISTEN_ADDR="127.0.0.1:3847"
readonly HEALTH_RETRIES=30
readonly HEALTH_SLEEP_SECONDS=1

log() {
  printf '[alpaca-mock-service] %s\n' "$*"
}

warn() {
  printf '[alpaca-mock-service] warning: %s\n' "$*" >&2
}

die() {
  printf '[alpaca-mock-service] error: %s\n' "$*" >&2
  exit 1
}

require_command() {
  local command_name="$1"
  command -v "$command_name" >/dev/null 2>&1 || die "missing required command: $command_name"
}

trim() {
  local value="$1"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

strip_optional_quotes() {
  local value="$1"
  if [[ ${#value} -ge 2 ]]; then
    if [[ "${value:0:1}" == '"' && "${value: -1}" == '"' ]]; then
      value="${value:1:${#value}-2}"
    elif [[ "${value:0:1}" == "'" && "${value: -1}" == "'" ]]; then
      value="${value:1:${#value}-2}"
    fi
  fi
  printf '%s' "$value"
}

dotenv_value() {
  local dotenv_path="$1"
  local key="$2"
  local line value

  while IFS= read -r line || [[ -n "$line" ]]; do
    line="$(trim "$line")"
    [[ -z "$line" || "${line:0:1}" == "#" ]] && continue
    if [[ "$line" =~ ^export[[:space:]]+ ]]; then
      line="${line#export}"
      line="$(trim "$line")"
    fi
    if [[ "$line" =~ ^${key}[[:space:]]*= ]]; then
      value="${line#*=}"
      value="${value%%#*}"
      value="$(trim "$value")"
      strip_optional_quotes "$value"
      return 0
    fi
  done <"$dotenv_path"

  return 1
}

dotenv_has_key() {
  local dotenv_path="$1"
  local key="$2"
  dotenv_value "$dotenv_path" "$key" >/dev/null 2>&1
}

xml_escape() {
  local value="$1"
  value="${value//&/&amp;}"
  value="${value//</&lt;}"
  value="${value//>/&gt;}"
  value="${value//\"/&quot;}"
  value="${value//\'/&apos;}"
  printf '%s' "$value"
}

health_addr_for() {
  local listen_addr="$1"
  local any4_prefix="0.0.0.0:"
  local any6_prefix="[::]:"

  if [[ "$listen_addr" == "$any4_prefix"* ]]; then
    printf '127.0.0.1:%s' "${listen_addr:${#any4_prefix}}"
  elif [[ "$listen_addr" == "$any6_prefix"* ]]; then
    printf '[::1]:%s' "${listen_addr:${#any6_prefix}}"
  else
    printf '%s' "$listen_addr"
  fi
}

ensure_path_supported_by_systemd_user_unit() {
  local path="$1"
  [[ "$path" != *[[:space:]]* ]] || die "systemd user service install does not support whitespace in this path: $path"
  [[ "$path" != *%* ]] || die "systemd user service install does not support '%' in this path: $path"
}

build_release_binary() {
  log "Building alpaca-mock release binary"
  cargo build --manifest-path "$REPO_ROOT/Cargo.toml" -p alpaca-mock --release
  [[ -x "$BINARY_PATH" ]] || die "release binary was not created at $BINARY_PATH"
}

write_launchd_plist() {
  local plist_path="$1"
  local log_dir="$2"
  local escaped_binary escaped_repo_root escaped_listen_addr escaped_stdout escaped_stderr

  escaped_binary="$(xml_escape "$BINARY_PATH")"
  escaped_repo_root="$(xml_escape "$REPO_ROOT")"
  escaped_listen_addr="$(xml_escape "$LISTEN_ADDR")"
  escaped_stdout="$(xml_escape "$log_dir/alpaca-mock.out.log")"
  escaped_stderr="$(xml_escape "$log_dir/alpaca-mock.err.log")"

  cat >"$plist_path" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${SERVICE_LABEL}</string>

  <key>ProgramArguments</key>
  <array>
    <string>${escaped_binary}</string>
  </array>

  <key>WorkingDirectory</key>
  <string>${escaped_repo_root}</string>

  <key>EnvironmentVariables</key>
  <dict>
    <key>ALPACA_MOCK_LISTEN_ADDR</key>
    <string>${escaped_listen_addr}</string>
  </dict>

  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>

  <key>StandardOutPath</key>
  <string>${escaped_stdout}</string>
  <key>StandardErrorPath</key>
  <string>${escaped_stderr}</string>
</dict>
</plist>
EOF
}

install_macos_service() {
  require_command launchctl

  local launch_agents_dir="$HOME/Library/LaunchAgents"
  local log_dir="$HOME/Library/Logs/alpaca-rust"
  local plist_path="$launch_agents_dir/${SERVICE_LABEL}.plist"
  local user_domain="gui/$(id -u)"

  mkdir -p "$launch_agents_dir" "$log_dir"
  write_launchd_plist "$plist_path" "$log_dir"

  log "Registering launchd service at $plist_path"
  launchctl bootout "$user_domain" "$plist_path" >/dev/null 2>&1 || true
  launchctl bootstrap "$user_domain" "$plist_path"
  launchctl kickstart -k "$user_domain/$SERVICE_LABEL"

  SERVICE_FILE_PATH="$plist_path"
  STATUS_COMMAND="launchctl print $user_domain/$SERVICE_LABEL"
  STOP_COMMAND="launchctl bootout $user_domain $plist_path"
  LOG_COMMAND="tail -f $log_dir/alpaca-mock.err.log"
}

write_systemd_user_unit() {
  local unit_path="$1"

  cat >"$unit_path" <<EOF
[Unit]
Description=alpaca-mock local service
Wants=network-online.target
After=network-online.target

[Service]
Type=simple
WorkingDirectory=${REPO_ROOT}
Environment=ALPACA_MOCK_LISTEN_ADDR=${LISTEN_ADDR}
ExecStart=${BINARY_PATH}
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF
}

install_ubuntu_service() {
  require_command systemctl

  if [[ -r /etc/os-release ]]; then
    # shellcheck disable=SC1091
    source /etc/os-release
    [[ "${ID:-}" == "ubuntu" ]] || die "unsupported Linux distribution: ${ID:-unknown}; only Ubuntu is supported"
  else
    die "could not read /etc/os-release; only Ubuntu is supported on Linux"
  fi

  ensure_path_supported_by_systemd_user_unit "$REPO_ROOT"
  ensure_path_supported_by_systemd_user_unit "$BINARY_PATH"

  if ! systemctl --user show-environment >/dev/null 2>&1; then
    die "systemd user session is not available; run this from a normal Ubuntu login session"
  fi

  local unit_dir="$HOME/.config/systemd/user"
  local unit_path="$unit_dir/$SYSTEMD_UNIT_NAME"
  mkdir -p "$unit_dir"
  write_systemd_user_unit "$unit_path"

  if command -v loginctl >/dev/null 2>&1; then
    if loginctl enable-linger "$(id -un)" >/dev/null 2>&1; then
      log "Enabled lingering for $(id -un) so the user service can survive logout"
    else
      warn "could not enable lingering for $(id -un); the service may stop after logout"
    fi
  fi

  log "Registering systemd user service at $unit_path"
  systemctl --user daemon-reload
  systemctl --user enable --now "$SYSTEMD_UNIT_NAME"
  systemctl --user restart "$SYSTEMD_UNIT_NAME"

  SERVICE_FILE_PATH="$unit_path"
  STATUS_COMMAND="systemctl --user status $SYSTEMD_UNIT_NAME"
  STOP_COMMAND="systemctl --user disable --now $SYSTEMD_UNIT_NAME"
  LOG_COMMAND="journalctl --user -u $SYSTEMD_UNIT_NAME -f"
}

wait_for_health() {
  local health_url="http://${HEALTH_ADDR}/health"
  local attempt health_body

  log "Waiting for health check at $health_url"
  for attempt in $(seq 1 "$HEALTH_RETRIES"); do
    if health_body="$(curl -fsS "$health_url" 2>/dev/null)" && [[ "$health_body" == *'"service":"alpaca-mock"'* ]]; then
      log "Health check passed: $health_url"
      return 0
    fi
    sleep "$HEALTH_SLEEP_SECONDS"
  done

  warn "health check did not pass after ${HEALTH_RETRIES}s"
  warn "status: $STATUS_COMMAND"
  warn "logs: $LOG_COMMAND"
  return 1
}

print_summary() {
  log "Service installed and running"
  log "Service file: $SERVICE_FILE_PATH"
  log "Base URL: http://${HEALTH_ADDR}"
  log "Status: $STATUS_COMMAND"
  log "Logs: $LOG_COMMAND"
  log "Stop: $STOP_COMMAND"
}

main() {
  local script_dir os_name

  script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
  REPO_ROOT="$(cd -- "$script_dir/.." && pwd)"
  ENV_FILE="$REPO_ROOT/.env"
  BINARY_PATH="$REPO_ROOT/target/release/alpaca-mock"
  SERVICE_FILE_PATH=""
  STATUS_COMMAND=""
  STOP_COMMAND=""
  LOG_COMMAND=""

  require_command cargo
  require_command curl
  require_command seq

  [[ "$EUID" -ne 0 ]] || die "run this script as the target user, not with sudo"
  [[ -f "$REPO_ROOT/Cargo.toml" ]] || die "could not find workspace Cargo.toml at $REPO_ROOT/Cargo.toml"
  [[ -f "$ENV_FILE" ]] || die "expected root .env at $ENV_FILE"

  dotenv_has_key "$ENV_FILE" "ALPACA_DATA_API_KEY" || die "expected ALPACA_DATA_API_KEY in $ENV_FILE"
  dotenv_has_key "$ENV_FILE" "ALPACA_DATA_SECRET_KEY" || die "expected ALPACA_DATA_SECRET_KEY in $ENV_FILE"

  LISTEN_ADDR="$(dotenv_value "$ENV_FILE" "ALPACA_MOCK_LISTEN_ADDR" || true)"
  if [[ -z "$LISTEN_ADDR" ]]; then
    LISTEN_ADDR="$DEFAULT_LISTEN_ADDR"
  fi
  HEALTH_ADDR="$(health_addr_for "$LISTEN_ADDR")"

  log "Workspace: $REPO_ROOT"
  log "Listen address: $LISTEN_ADDR"

  build_release_binary

  os_name="$(uname -s)"
  case "$os_name" in
    Darwin)
      install_macos_service
      ;;
    Linux)
      install_ubuntu_service
      ;;
    *)
      die "unsupported operating system: $os_name; only macOS and Ubuntu are supported"
      ;;
  esac

  wait_for_health
  print_summary
}

main "$@"
