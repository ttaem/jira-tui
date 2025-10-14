#!/bin/bash

# JIRA TUI 실행 스크립트

# 환경 변수 확인
if [ -z "$JIRA_EMAIL" ]; then
    echo "❌ Error: JIRA_EMAIL 환경 변수가 설정되지 않았습니다."
    echo "예시: export JIRA_EMAIL=your.email@newracom.com"
    exit 1
fi

if [ -z "$JIRA_API_TOKEN" ]; then
    echo "❌ Error: JIRA_API_TOKEN 환경 변수가 설정되지 않았습니다."
    echo "API 토큰 생성: https://id.atlassian.com/manage-profile/security/api-tokens"
    exit 1
fi

# 기본 JIRA URL 설정 (필요시 변경)
export JIRA_BASE_URL="${JIRA_BASE_URL:-https://newracom.atlassian.net}"

echo "🚀 JIRA TUI 시작 중..."
echo "📧 Email: $JIRA_EMAIL"
echo "🌐 JIRA URL: $JIRA_BASE_URL"
echo ""

# 프로그램 실행
./target/release/jira-tui