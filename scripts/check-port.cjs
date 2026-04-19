/**
 * Check if port 1420 is in use before starting dev server.
 * If occupied, prints the process name and asks to terminate.
 */
const { execSync } = require('child_process');
const readline = require('readline');

const PORT = 1420;

function getProcessOnPort() {
  try {
    const output = execSync(
      `netstat -ano | findstr :${PORT} | findstr LISTENING`,
      { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
    const lines = output.trim().split('\n');
    if (lines.length === 0 || !lines[0].trim()) return null;

    // Extract PID from last column
    const parts = lines[0].trim().split(/\s+/);
    const pid = parseInt(parts[parts.length - 1], 10);
    if (!pid || isNaN(pid)) return null;

    // Get process name from PID
    let name = 'unknown';
    try {
      const taskInfo = execSync(
        `tasklist /FI "PID eq ${pid}" /FO CSV /NH`,
        { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
      );
      const match = taskInfo.match(/"([^"]+)"/);
      if (match) name = match[1];
    } catch { /* ignore */ }

    return { pid, name };
  } catch {
    return null;
  }
}

function ask(question) {
  // In non-interactive mode (piped stdin), default to "yes"
  if (!process.stdin.isTTY) return Promise.resolve('y');
  const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer.trim().toLowerCase());
    });
  });
}

async function main() {
  const proc = getProcessOnPort();
  if (!proc) {
    // Port is free
    process.exit(0);
  }

  console.log(`\n⚠️  Port ${PORT} is already in use!`);
  console.log(`   Process: ${proc.name} (PID ${proc.pid})\n`);

  const answer = await ask(`   Terminate ${proc.name} (PID ${proc.pid})? [Y/n] `);

  if (answer === '' || answer === 'y' || answer === 'yes') {
    try {
      execSync(`taskkill /PID ${proc.pid} /F`, { stdio: 'pipe' });
      console.log(`   ✓ Killed ${proc.name} (PID ${proc.pid})\n`);
    } catch (e) {
      console.error(`   ✗ Failed to kill process: ${e.message}`);
      process.exit(1);
    }
  } else {
    console.log('   Aborted. Cannot start dev server while port is occupied.');
    process.exit(1);
  }
}

main();
