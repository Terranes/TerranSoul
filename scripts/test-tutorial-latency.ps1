# Tutorial latency test - measures TTFT (embed + prompt_eval) for each step
# TTFT = Time To First Token = embed query + LLM prompt evaluation
# This is what the user experiences as "delay" before text starts streaming

$ollamaUrl = "http://localhost:11434"

# Warm models
Write-Host "Warming models..."
Invoke-RestMethod -Uri "$ollamaUrl/api/embed" -Method Post -ContentType "application/json" -Body '{"model":"nomic-embed-text","input":"warmup","keep_alive":"30m"}' | Out-Null
Invoke-RestMethod -Uri "$ollamaUrl/api/chat" -Method Post -ContentType "application/json" -Body '{"model":"gemma4:e4b","messages":[{"role":"user","content":"hi"}],"stream":false,"options":{"num_predict":1},"keep_alive":"30m"}' | Out-Null
Write-Host "Models warm.`n"

# Test queries from tutorial steps
$steps = @(
    @{step="Step 9  (EN)"; q="What is the statute of limitations for contract disputes under Vietnamese law?"; eq="What is the statute of limitations for contract disputes under Vietnamese law?"},
    @{step="Step 10 (EN)"; q="Can a party claim both a penalty and damages for breach of contract?"; eq="Can a party claim both a penalty and damages for breach of contract?"},
    @{step="Step 11 (VN)"; q="Thoi hieu khoi kien tranh chap hop dong theo phap luat Viet Nam la bao lau?"; eq="statute of limitations contract dispute Vietnam"},
    @{step="Step 12 (CN)"; q="Statute of limitations for contract disputes under Vietnamese law?"; eq="contract dispute statute limitations Chinese"},
    @{step="Step 13 (RU)"; q="What is the limitation period for contract disputes?"; eq="limitation period contract disputes Russian"},
    @{step="Step 14 (JP)"; q="Contract dispute statute of limitations Vietnam?"; eq="contract dispute limitation Japanese"},
    @{step="Step 15 (KR)"; q="Contract dispute limitation period Korean?"; eq="contract dispute limitation Korean"},
    @{step="Step 18 (EN)"; q="Summarize what you know about me and my documents."; eq="Summarize what you know about me and my documents"}
)

Write-Host "============================================="
Write-Host "  Tutorial Latency Test (TTFT = Time to First Token)"
Write-Host "  Target: < 1000ms TTFT for each step"
Write-Host "============================================="
Write-Host ""

$allPass = $true

foreach ($s in $steps) {
    # 1. Embed query
    $embedBody = @{ model = "nomic-embed-text"; input = $s.eq; keep_alive = "30m" } | ConvertTo-Json -Compress
    $embedTime = Measure-Command {
        Invoke-RestMethod -Uri "$ollamaUrl/api/embed" -Method Post -ContentType "application/json" -Body $embedBody | Out-Null
    }

    # 2. LLM with RAG context
    $chatPayload = @{
        model = "gemma4:e4b"
        messages = @(
            @{ role = "system"; content = "You are TerranSoul, a helpful AI companion. Reply concisely.`n`n[LONG-TERM MEMORY]`n- Article 429 of the 2015 Vietnamese Civil Code: Statute of limitations for contract disputes is 3 years from when claimant knew or should have known of breach.`n- Article 351: Strict liability - no need to prove fault for breach of obligation.`n- Article 352: Full compensation for breach of obligation.`n- Article 420: Penalty clauses - may claim both penalty AND damages for breach.`n- Article 419: Material + spiritual losses including lost benefits.`n- Article 421: Exemption in force majeure cases.`n[/LONG-TERM MEMORY]" },
            @{ role = "user"; content = $s.q }
        )
        stream = $false
        options = @{ num_predict = 80 }
        keep_alive = "30m"
    } | ConvertTo-Json -Depth 4 -Compress
    $chatResp = Invoke-RestMethod -Uri "$ollamaUrl/api/chat" -Method Post -ContentType "application/json" -Body ([System.Text.Encoding]::UTF8.GetBytes($chatPayload))

    $embedMs = [int]$embedTime.TotalMilliseconds
    $promptMs = [int]($chatResp.prompt_eval_duration / 1e6)
    $genMs = [int]($chatResp.eval_duration / 1e6)
    $tokCount = $chatResp.eval_count
    $ttft = $embedMs + $promptMs
    $pass = if ($ttft -lt 1000) { "PASS" } else { "FAIL"; $allPass = $false }

    Write-Host "$($s.step): embed=${embedMs}ms + prompt=${promptMs}ms = TTFT ${ttft}ms [$pass] (gen=${genMs}ms/${tokCount}tok)"
}

Write-Host ""
Write-Host "============================================="
if ($allPass) {
    Write-Host "  ALL STEPS PASS - TTFT < 1s"
} else {
    Write-Host "  SOME STEPS FAILED - review above"
}
Write-Host "============================================="
