---
name: security-audit
description: Performs a security audit on code or configuration. Checks for OWASP Top 10 vulnerabilities, secrets exposure, and unsafe patterns.
triggers:
  - security audit
  - check for vulnerabilities
  - is this secure
  - security review
  - find security issues
---

# Security Audit Skill

## Objective

Identify security vulnerabilities in code, configuration, and dependencies following OWASP Top 10 and secure coding practices.

## Checklist

### 1. Injection (SQL, Command, XSS)
- Search for string concatenation in queries: `code_search` for `format!`, `+` in SQL strings
- Check for unsanitized user input passed to `shell_exec`, `eval`, or database queries
- Look for innerHTML/dangerouslySetInnerHTML without sanitization

### 2. Authentication & Session
- Check for hardcoded credentials: `code_search` for "password", "secret", "api_key", "token"
- Verify session tokens use secure random generation
- Check for missing auth checks on sensitive endpoints

### 3. Sensitive Data Exposure
- Search for `.env`, credentials, private keys in tracked files
- Check for sensitive data in logs or error messages
- Verify secrets aren't in client-side code

### 4. Access Control
- Check for missing authorization on API endpoints
- Look for IDOR (direct object reference) vulnerabilities
- Verify file access is sandboxed

### 5. Security Misconfiguration
- Check CORS policies
- Verify TLS/HTTPS usage
- Look for debug mode enabled in production
- Check dependency versions for known CVEs

### 6. Path Traversal
- Search for file operations using user input
- Check for `../` handling and path canonicalization

## Tools Used

- `code_search` — find vulnerable patterns
- `file_read` — inspect configuration files
- `file_search` — find secrets in code
- `web_search` — look up CVEs for dependencies
- `shell_exec` — run security scanning tools if available

## Output Format

```
## Security Audit Report

### CRITICAL
- [File:Line] Vulnerability description | OWASP category | Remediation

### HIGH
- [File:Line] Issue description | Category | Fix

### MEDIUM / LOW
- [File:Line] Finding | Category | Suggestion

### Summary
- X critical, Y high, Z medium vulnerabilities
- Risk assessment: HIGH / MEDIUM / LOW
```
