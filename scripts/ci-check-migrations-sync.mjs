#!/usr/bin/env node
/**
 * CI check: Verify that new migrations in mcp-data/shared/migrations/
 * have corresponding updates to mcp-data/shared/lessons-learned.md.
 *
 * Rule (from chunk 46.1): When a new mcp-data/shared/migrations/NNN_*.sql
 * is added, mcp-data/shared/lessons-learned.md must be updated in the same commit.
 *
 * This script:
 * 1. Finds all .sql files in mcp-data/shared/migrations/
 * 2. For each migration, reads the first INSERT statement to extract the lesson ID
 * 3. Checks if that lesson content appears in lessons-learned.md
 * 4. Fails if a migration exists but has no corresponding lessons-learned entry
 */

import fs from 'fs';
import path from 'path';

const MIGRATIONS_DIR = 'mcp-data/shared/migrations';
const LESSONS_FILE = 'mcp-data/shared/lessons-learned.md';

// Extract the first INSERT statement from a migration SQL file
function extractLessonIdFromSql(sqlContent) {
  const match = sqlContent.match(/INSERT\s+INTO\s+memories\s*\([^)]*content[^)]*\)\s*\n\s*SELECT\s*\n\s*'([^']+)'/is);
  if (match && match[1]) {
    return match[1].substring(0, 100); // First 100 chars as unique key
  }
  return null;
}

// Main check
try {
  if (!fs.existsSync(MIGRATIONS_DIR)) {
    console.log('✓ No migrations directory yet.');
    process.exit(0);
  }

  const migrations = fs.readdirSync(MIGRATIONS_DIR)
    .filter(f => f.endsWith('.sql') && f.match(/^\d{3}_/))
    .sort();

  if (migrations.length === 0) {
    console.log('✓ No SQL migrations found.');
    process.exit(0);
  }

  const lessonsContent = fs.existsSync(LESSONS_FILE)
    ? fs.readFileSync(LESSONS_FILE, 'utf-8')
    : '';

  let allSynced = true;

  for (const migration of migrations) {
    const sqlPath = path.join(MIGRATIONS_DIR, migration);
    const sqlContent = fs.readFileSync(sqlPath, 'utf-8');

    // Extract lesson identifier
    const lessonId = extractLessonIdFromSql(sqlContent);
    if (!lessonId) {
      console.warn(`⚠ ${migration}: Could not extract lesson ID from INSERT statement`);
      continue;
    }

    // Check if this lesson appears in lessons-learned.md
    if (lessonsContent.includes(lessonId)) {
      console.log(`✓ ${migration}: Found in lessons-learned.md`);
    } else {
      console.error(
        `✗ ${migration}: Migration exists but NO corresponding entry in lessons-learned.md`
      );
      console.error(`   Expected to find: "${lessonId.substring(0, 50)}..."`);
      allSynced = false;
    }
  }

  if (!allSynced) {
    console.error('\n❌ CI check failed: Migrations are out of sync with lessons-learned.md');
    console.error('   Add a section to lessons-learned.md documenting each new migration.');
    process.exit(1);
  } else {
    console.log('\n✅ All migrations are in sync with lessons-learned.md');
    process.exit(0);
  }
} catch (err) {
  console.error('CI check error:', err.message);
  process.exit(1);
}
