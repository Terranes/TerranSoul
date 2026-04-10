<template>
  <div class="typing-indicator">
    <span class="dot" />
    <span class="dot" />
    <span class="dot" />
  </div>
</template>

<script setup lang="ts">
</script>

<style scoped>
.typing-indicator {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 10px 14px;
  width: fit-content;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 16px;
  border-bottom-left-radius: 4px;
  margin-left: 4px;
}

.dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #a89cff;
  animation: bounce 1.2s infinite ease-in-out;
}

.dot:nth-child(1) { animation-delay: 0s; }
.dot:nth-child(2) { animation-delay: 0.2s; }
.dot:nth-child(3) { animation-delay: 0.4s; }

@keyframes bounce {
  0%, 60%, 100% { transform: translateY(0); opacity: 0.4; }
  30% { transform: translateY(-5px); opacity: 1; }
}
</style>
