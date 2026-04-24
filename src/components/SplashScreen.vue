<template>
  <div class="splash">
    <div class="splash-bg" />

    <!-- Floating particles background -->
    <div class="particles">
      <div
        v-for="i in 12"
        :key="i"
        class="particle"
        :class="`p${i}`"
      />
    </div>

    <!-- Cute anime cat lying on the floor — kawaii SVG art -->
    <div class="cat-wrapper">
      <svg
        class="cat-svg"
        viewBox="0 0 320 200"
        xmlns="http://www.w3.org/2000/svg"
      >
        <defs>
          <!-- Big anime eye gradient — golden amber -->
          <radialGradient
            id="eyeGrad"
            cx="40%"
            cy="28%"
          >
            <stop
              offset="0%"
              stop-color="#ffe9a0"
            />
            <stop
              offset="35%"
              stop-color="#f5c842"
            />
            <stop
              offset="70%"
              stop-color="#e8a020"
            />
            <stop
              offset="100%"
              stop-color="#c87818"
            />
          </radialGradient>
          <!-- Soft fur body gradient — warm cream-white -->
          <radialGradient
            id="bodyGlow"
            cx="45%"
            cy="30%"
          >
            <stop
              offset="0%"
              stop-color="#fefefe"
            />
            <stop
              offset="60%"
              stop-color="#f5f0e8"
            />
            <stop
              offset="100%"
              stop-color="#e8e0d4"
            />
          </radialGradient>
          <!-- Head fur gradient -->
          <radialGradient
            id="headFur"
            cx="42%"
            cy="25%"
          >
            <stop
              offset="0%"
              stop-color="#ffffff"
            />
            <stop
              offset="100%"
              stop-color="#f0eae0"
            />
          </radialGradient>
          <!-- Blush -->
          <radialGradient id="blushGrad">
            <stop
              offset="0%"
              stop-color="rgba(255,140,160,0.55)"
            />
            <stop
              offset="100%"
              stop-color="rgba(255,140,160,0)"
            />
          </radialGradient>
          <!-- Sparkle glow filter -->
          <filter id="glow">
            <feGaussianBlur
              stdDeviation="2"
              result="blur"
            />
            <feMerge><feMergeNode in="blur" /><feMergeNode in="SourceGraphic" /></feMerge>
          </filter>
          <!-- Soft warm glow behind cat -->
          <radialGradient
            id="warmGlow"
            cx="50%"
            cy="50%"
          >
            <stop
              offset="0%"
              stop-color="rgba(255,220,160,0.12)"
            />
            <stop
              offset="100%"
              stop-color="rgba(255,220,160,0)"
            />
          </radialGradient>
          <!-- Shadow under body -->
          <radialGradient id="shadowGrad">
            <stop
              offset="0%"
              stop-color="rgba(0,0,0,0.15)"
            />
            <stop
              offset="100%"
              stop-color="rgba(0,0,0,0)"
            />
          </radialGradient>
          <!-- Ear pink gradient -->
          <linearGradient
            id="earPink"
            x1="0%"
            y1="0%"
            x2="100%"
            y2="100%"
          >
            <stop
              offset="0%"
              stop-color="#ffc4d0"
            />
            <stop
              offset="100%"
              stop-color="#ffaabb"
            />
          </linearGradient>
        </defs>

        <!-- Warm ambient glow -->
        <ellipse
          cx="155"
          cy="120"
          rx="140"
          ry="90"
          fill="url(#warmGlow)"
        />

        <!-- Ground shadow — elongated for lying pose -->
        <ellipse
          cx="160"
          cy="182"
          rx="100"
          ry="10"
          fill="url(#shadowGrad)"
        />

        <!-- ── Tail — fluffy, curled up behind body ── -->
        <g class="tail-group">
          <path
            d="M255,138 Q280,120 290,95 Q296,78 285,68 Q275,60 268,70"
            stroke="#e8e0d4"
            stroke-width="18"
            stroke-linecap="round"
            fill="none"
          />
          <path
            d="M255,138 Q280,120 290,95 Q296,78 285,68 Q275,60 268,70"
            stroke="#f5f0e8"
            stroke-width="13"
            stroke-linecap="round"
            fill="none"
          />
          <!-- Tail tip fluff -->
          <ellipse
            cx="268"
            cy="68"
            rx="12"
            ry="10"
            fill="#f8f4ee"
          />
        </g>

        <!-- ── Back legs — tucked under body ── -->
        <ellipse
          cx="220"
          cy="170"
          rx="22"
          ry="12"
          fill="#eee8de"
        />
        <ellipse
          cx="220"
          cy="170"
          rx="20"
          ry="10"
          fill="#f5f0e8"
        />
        <!-- Back paw peeking out -->
        <ellipse
          cx="238"
          cy="174"
          rx="10"
          ry="7"
          fill="#faf6f0"
        />
        <!-- Back paw beans -->
        <ellipse
          cx="235"
          cy="176"
          rx="2.5"
          ry="2"
          fill="rgba(255,170,185,0.5)"
        />
        <ellipse
          cx="240"
          cy="176"
          rx="2.5"
          ry="2"
          fill="rgba(255,170,185,0.5)"
        />

        <!-- ── Body — long oval, lying flat ── -->
        <g class="body-group">
          <ellipse
            cx="170"
            cy="145"
            rx="72"
            ry="38"
            fill="url(#bodyGlow)"
          />
          <!-- Belly — lighter, softer underbelly showing -->
          <ellipse
            cx="160"
            cy="155"
            rx="50"
            ry="22"
            fill="#ffffff"
            opacity="0.6"
          />
          <!-- Subtle fur texture lines -->
          <path
            d="M130,135 Q145,130 160,133"
            stroke="rgba(210,200,185,0.3)"
            stroke-width="0.7"
            fill="none"
          />
          <path
            d="M145,140 Q165,136 185,140"
            stroke="rgba(210,200,185,0.25)"
            stroke-width="0.7"
            fill="none"
          />
          <path
            d="M140,150 Q160,146 180,150"
            stroke="rgba(210,200,185,0.2)"
            stroke-width="0.7"
            fill="none"
          />
        </g>

        <!-- ── Front paws — stretched out forward adorably ── -->
        <!-- Left front leg -->
        <g class="paw-left">
          <path
            d="M110,152 Q90,158 72,166"
            stroke="#f0ebe4"
            stroke-width="16"
            stroke-linecap="round"
            fill="none"
          />
          <path
            d="M110,152 Q90,158 72,166"
            stroke="#f8f4ee"
            stroke-width="12"
            stroke-linecap="round"
            fill="none"
          />
          <ellipse
            cx="68"
            cy="168"
            rx="14"
            ry="9"
            fill="#faf6f0"
          />
          <!-- Toe beans -->
          <ellipse
            cx="60"
            cy="170"
            rx="3"
            ry="2.2"
            fill="rgba(255,165,185,0.55)"
          />
          <ellipse
            cx="65"
            cy="172"
            rx="3"
            ry="2.2"
            fill="rgba(255,165,185,0.55)"
          />
          <ellipse
            cx="71"
            cy="172"
            rx="3"
            ry="2.2"
            fill="rgba(255,165,185,0.55)"
          />
          <!-- Big center pad -->
          <ellipse
            cx="66"
            cy="166"
            rx="4.5"
            ry="3"
            fill="rgba(255,155,175,0.45)"
          />
        </g>
        <!-- Right front leg -->
        <g class="paw-right">
          <path
            d="M118,155 Q100,162 85,172"
            stroke="#f0ebe4"
            stroke-width="14"
            stroke-linecap="round"
            fill="none"
          />
          <path
            d="M118,155 Q100,162 85,172"
            stroke="#f8f4ee"
            stroke-width="10"
            stroke-linecap="round"
            fill="none"
          />
          <ellipse
            cx="82"
            cy="174"
            rx="13"
            ry="8"
            fill="#faf6f0"
          />
          <!-- Toe beans -->
          <ellipse
            cx="75"
            cy="176"
            rx="2.8"
            ry="2"
            fill="rgba(255,165,185,0.5)"
          />
          <ellipse
            cx="80"
            cy="177"
            rx="2.8"
            ry="2"
            fill="rgba(255,165,185,0.5)"
          />
          <ellipse
            cx="86"
            cy="177"
            rx="2.8"
            ry="2"
            fill="rgba(255,165,185,0.5)"
          />
        </g>

        <!-- ── Head — big round anime proportions, resting on paws ── -->
        <g class="head-group">
          <!-- Neck connection -->
          <ellipse
            cx="125"
            cy="128"
            rx="28"
            ry="22"
            fill="#f2ece4"
          />

          <!-- Head shape -->
          <ellipse
            cx="118"
            cy="100"
            rx="50"
            ry="46"
            fill="url(#headFur)"
          />
          <!-- Head outline hint -->
          <ellipse
            cx="118"
            cy="100"
            rx="50"
            ry="46"
            fill="none"
            stroke="rgba(200,190,175,0.2)"
            stroke-width="0.5"
          />

          <!-- ── Ears — tall pointed, slightly tilted ── -->
          <!-- Left ear -->
          <g class="ear-left">
            <polygon
              points="82,72 62,22 100,58"
              fill="#f0eae0"
            />
            <polygon
              points="84,68 68,30 96,60"
              fill="url(#earPink)"
            />
            <polygon
              points="86,64 72,36 93,58"
              fill="#ffaabb"
            />
          </g>
          <!-- Right ear -->
          <g class="ear-right">
            <polygon
              points="148,68 166,20 130,55"
              fill="#f0eae0"
            />
            <polygon
              points="146,64 160,28 133,57"
              fill="url(#earPink)"
            />
            <polygon
              points="144,60 156,34 136,55"
              fill="#ffaabb"
            />
          </g>

          <!-- Forehead fluff tuft -->
          <path
            d="M108,62 Q115,54 118,62 Q121,54 128,62"
            stroke="#f8f4ee"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
          />

          <!-- White muzzle area -->
          <ellipse
            cx="118"
            cy="116"
            rx="24"
            ry="18"
            fill="#ffffff"
          />

          <!-- ── Eyes — big sparkly anime, sleepy-happy ── -->
          <g class="eyes">
            <!-- Left eye -->
            <ellipse
              cx="98"
              cy="96"
              rx="14"
              ry="16"
              fill="#1a1420"
            />
            <ellipse
              cx="98"
              cy="96"
              rx="12"
              ry="14"
              fill="url(#eyeGrad)"
            />
            <!-- Pupil — large, round -->
            <ellipse
              cx="98"
              cy="98"
              rx="6"
              ry="10"
              fill="#1a0e08"
            />
            <!-- Large highlight -->
            <circle
              cx="104"
              cy="90"
              r="5"
              fill="white"
              opacity="0.95"
              filter="url(#glow)"
            />
            <!-- Secondary highlight -->
            <circle
              cx="93"
              cy="102"
              r="2.8"
              fill="white"
              opacity="0.75"
            />
            <!-- Tiny sparkle -->
            <circle
              cx="101"
              cy="95"
              r="1.4"
              fill="white"
              opacity="0.5"
            />
            <!-- Bottom lid line — happy squint -->
            <path
              d="M86,104 Q98,108 110,104"
              stroke="#1a1420"
              stroke-width="1.5"
              fill="none"
              stroke-linecap="round"
            />

            <!-- Right eye -->
            <ellipse
              cx="138"
              cy="96"
              rx="14"
              ry="16"
              fill="#1a1420"
            />
            <ellipse
              cx="138"
              cy="96"
              rx="12"
              ry="14"
              fill="url(#eyeGrad)"
            />
            <!-- Pupil -->
            <ellipse
              cx="138"
              cy="98"
              rx="6"
              ry="10"
              fill="#1a0e08"
            />
            <!-- Large highlight -->
            <circle
              cx="144"
              cy="90"
              r="5"
              fill="white"
              opacity="0.95"
              filter="url(#glow)"
            />
            <!-- Secondary highlight -->
            <circle
              cx="133"
              cy="102"
              r="2.8"
              fill="white"
              opacity="0.75"
            />
            <!-- Tiny sparkle -->
            <circle
              cx="141"
              cy="95"
              r="1.4"
              fill="white"
              opacity="0.5"
            />
            <!-- Bottom lid line -->
            <path
              d="M126,104 Q138,108 150,104"
              stroke="#1a1420"
              stroke-width="1.5"
              fill="none"
              stroke-linecap="round"
            />
          </g>

          <!-- Blush cheeks — rosy pink -->
          <ellipse
            cx="80"
            cy="110"
            rx="12"
            ry="6.5"
            fill="url(#blushGrad)"
          />
          <ellipse
            cx="156"
            cy="110"
            rx="12"
            ry="6.5"
            fill="url(#blushGrad)"
          />

          <!-- Small pink nose -->
          <path
            d="M114,110 L118,106 L122,110 Z"
            fill="#f48a9e"
          />
          <ellipse
            cx="118"
            cy="107.5"
            rx="2.2"
            ry="1.1"
            fill="rgba(255,255,255,0.4)"
          />

          <!-- Mouth — happy :3 with a tiny fang -->
          <path
            d="M110,114 Q114,119 118,114 Q122,119 126,114"
            stroke="#d0889a"
            stroke-width="1.6"
            fill="none"
            stroke-linecap="round"
          />
          <!-- Tiny fang peeking out -->
          <path
            d="M114,114 L113,117.5"
            stroke="#ffffff"
            stroke-width="1.3"
            stroke-linecap="round"
          />

          <!-- Whiskers — delicate, slightly droopy (relaxed) -->
          <g
            class="whiskers"
            stroke="rgba(180,170,155,0.4)"
            stroke-width="0.9"
            stroke-linecap="round"
          >
            <line
              x1="76"
              y1="106"
              x2="38"
              y2="100"
            />
            <line
              x1="74"
              y1="112"
              x2="34"
              y2="112"
            />
            <line
              x1="76"
              y1="118"
              x2="40"
              y2="126"
            />
            <line
              x1="160"
              y1="106"
              x2="198"
              y2="100"
            />
            <line
              x1="162"
              y1="112"
              x2="202"
              y2="112"
            />
            <line
              x1="160"
              y1="118"
              x2="196"
              y2="126"
            />
          </g>
        </g>
      </svg>

      <!-- Floating sparkles around the cat -->
      <div class="sparkle s1">
        ✦
      </div>
      <div class="sparkle s2">
        ✧
      </div>
      <div class="sparkle s3">
        ✦
      </div>
      <div class="sparkle s4">
        ⋆
      </div>
      <div class="sparkle s5">
        ✧
      </div>
      <div class="sparkle s6">
        ⋆
      </div>

      <!-- Sleeping Z's when idle for a while -->
      <div class="zzz">
        <span class="z z1">z</span>
        <span class="z z2">z</span>
        <span class="z z3">Z</span>
      </div>
    </div>

    <!-- Loading text with glow -->
    <div class="splash-text">
      <span class="splash-title">TerranSoul</span>
      <span class="splash-dots">
        <span class="dot">.</span><span class="dot">.</span><span class="dot">.</span>
      </span>
    </div>
    <div class="splash-sub">
      Waking up…
    </div>
  </div>
</template>

<style scoped>
.splash {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  background: #0b1120;
  gap: 1.5rem;
  overflow: hidden;
}

.splash-bg {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(ellipse at 50% 45%, rgba(120, 100, 255, 0.08) 0%, transparent 65%),
    radial-gradient(ellipse at 30% 65%, rgba(200, 140, 255, 0.06) 0%, transparent 55%),
    radial-gradient(ellipse at 70% 30%, rgba(255, 200, 120, 0.04) 0%, transparent 50%);
}

/* ── Floating background particles ── */
.particles {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.particle {
  position: absolute;
  border-radius: 50%;
  animation: particleDrift linear infinite;
}
.p1  { width: 3px; height: 3px; background: rgba(180,160,255,0.4); top: 15%; left: 10%; animation-duration: 8s; }
.p2  { width: 2px; height: 2px; background: rgba(255,200,160,0.3); top: 25%; left: 80%; animation-duration: 10s; animation-delay: 1s; }
.p3  { width: 4px; height: 4px; background: rgba(160,220,255,0.25); top: 60%; left: 15%; animation-duration: 12s; animation-delay: 2s; }
.p4  { width: 2px; height: 2px; background: rgba(255,180,200,0.35); top: 70%; left: 70%; animation-duration: 9s; animation-delay: 0.5s; }
.p5  { width: 3px; height: 3px; background: rgba(200,255,200,0.2); top: 40%; left: 90%; animation-duration: 11s; animation-delay: 3s; }
.p6  { width: 2px; height: 2px; background: rgba(255,220,160,0.3); top: 80%; left: 40%; animation-duration: 7s; animation-delay: 1.5s; }
.p7  { width: 3px; height: 3px; background: rgba(180,180,255,0.3); top: 10%; left: 55%; animation-duration: 13s; animation-delay: 4s; }
.p8  { width: 2px; height: 2px; background: rgba(255,160,200,0.25); top: 50%; left: 30%; animation-duration: 10s; animation-delay: 2.5s; }
.p9  { width: 4px; height: 4px; background: rgba(160,200,255,0.2); top: 30%; left: 45%; animation-duration: 14s; animation-delay: 1s; }
.p10 { width: 2px; height: 2px; background: rgba(220,180,255,0.3); top: 85%; left: 60%; animation-duration: 8s; animation-delay: 3.5s; }
.p11 { width: 3px; height: 3px; background: rgba(255,200,180,0.2); top: 20%; left: 35%; animation-duration: 11s; animation-delay: 0s; }
.p12 { width: 2px; height: 2px; background: rgba(180,255,220,0.25); top: 55%; left: 85%; animation-duration: 9s; animation-delay: 2s; }

@keyframes particleDrift {
  0%   { transform: translateY(0) translateX(0) scale(1); opacity: 0; }
  10%  { opacity: 1; }
  90%  { opacity: 1; }
  100% { transform: translateY(-80px) translateX(30px) scale(0.5); opacity: 0; }
}

/* ── SVG Anime Cat (lying on floor) ── */
.cat-wrapper {
  position: relative;
  width: 280px;
  height: 180px;
}

.cat-svg {
  width: 100%;
  height: 100%;
  overflow: visible;
}

/* Gentle breathing — subtle body rise and fall */
.body-group {
  animation: breathe 3.5s ease-in-out infinite;
  transform-origin: 170px 155px;
}
@keyframes breathe {
  0%, 100% { transform: scaleY(1) translateY(0); }
  50% { transform: scaleY(1.025) translateY(-1px); }
}

/* Head gentle bob — resting, dreamy */
.head-group {
  animation: headBob 4s ease-in-out infinite;
  transform-origin: 118px 120px;
}
@keyframes headBob {
  0%, 100% { transform: translateY(0) rotate(0deg); }
  40% { transform: translateY(-2px) rotate(-0.5deg); }
  70% { transform: translateY(-1px) rotate(0.3deg); }
}

/* Blink animation — double blink pattern */
.eyes {
  animation: blink 5s ease-in-out infinite;
  transform-origin: 118px 96px;
}
@keyframes blink {
  0%, 38%, 44%, 100% { transform: scaleY(1); }
  40% { transform: scaleY(0.05); }
  42% { transform: scaleY(1); }
  56% { transform: scaleY(0.05); }
  58%, 60% { transform: scaleY(1); }
}

/* Tail swish — lazy, relaxed movement */
.tail-group {
  animation: tailLazy 3.5s ease-in-out infinite;
  transform-origin: 255px 138px;
}
@keyframes tailLazy {
  0%, 100% { transform: rotate(0deg); }
  25% { transform: rotate(-6deg); }
  50% { transform: rotate(3deg); }
  75% { transform: rotate(-3deg); }
}

/* Left ear twitch — occasional curious twitch */
.ear-left {
  animation: earTwitchL 6s ease-in-out infinite;
  transform-origin: 82px 58px;
}
@keyframes earTwitchL {
  0%, 80%, 88%, 100% { transform: rotate(0deg); }
  83% { transform: rotate(-8deg); }
  86% { transform: rotate(2deg); }
}

/* Right ear — different timing for natural feel */
.ear-right {
  animation: earTwitchR 7s ease-in-out infinite;
  transform-origin: 148px 55px;
}
@keyframes earTwitchR {
  0%, 60%, 68%, 100% { transform: rotate(0deg); }
  63% { transform: rotate(6deg); }
  66% { transform: rotate(-1deg); }
}

/* Left paw — gentle kneading stretch */
.paw-left {
  animation: pawStretchL 4s ease-in-out infinite;
  transform-origin: 100px 155px;
}
@keyframes pawStretchL {
  0%, 100% { transform: translateX(0) translateY(0); }
  50% { transform: translateX(-3px) translateY(-1px); }
}

/* Right paw — offset timing */
.paw-right {
  animation: pawStretchR 4.5s ease-in-out infinite;
  transform-origin: 110px 160px;
}
@keyframes pawStretchR {
  0%, 100% { transform: translateX(0) translateY(0); }
  50% { transform: translateX(-2px) translateY(-1.5px); }
}

/* Whisker twitch */
.whiskers {
  animation: whiskerTwitch 3s ease-in-out infinite;
  transform-origin: 118px 112px;
}
@keyframes whiskerTwitch {
  0%, 85%, 92%, 100% { transform: scaleX(1); }
  88% { transform: scaleX(1.04); }
}

/* ── Sparkles ── */
.sparkle {
  position: absolute;
  font-size: 16px;
  animation: sparkleFloat 3s ease-in-out infinite;
  pointer-events: none;
}
.s1 { top: -5px; left: 20px; animation-delay: 0s; color: rgba(180,160,255,0.7); font-size: 18px; }
.s2 { top: -10px; right: 30px; animation-delay: 1.2s; color: rgba(255,200,140,0.7); }
.s3 { top: 40px; right: -10px; animation-delay: 2.4s; font-size: 12px; color: rgba(255,160,200,0.7); }
.s4 { top: 50px; left: -8px; animation-delay: 0.8s; font-size: 11px; color: rgba(160,230,200,0.6); }
.s5 { bottom: 20px; left: 30px; animation-delay: 1.8s; font-size: 14px; color: rgba(200,180,255,0.5); }
.s6 { bottom: 10px; right: 50px; animation-delay: 3s; font-size: 10px; color: rgba(255,220,160,0.6); }

@keyframes sparkleFloat {
  0%, 100% { opacity: 0; transform: translateY(0) scale(0.5) rotate(0deg); }
  15% { opacity: 1; }
  50% { opacity: 1; transform: translateY(-14px) scale(1.2) rotate(20deg); }
  85% { opacity: 1; }
}

/* ── Sleeping Z's ── */
.zzz {
  position: absolute;
  top: -10px;
  right: 50px;
  pointer-events: none;
}
.z {
  position: absolute;
  font-family: 'Inter', system-ui, sans-serif;
  font-weight: 700;
  font-style: italic;
  color: rgba(180,200,255,0.6);
  animation: zFloat 3s ease-in-out infinite;
}
.z1 { font-size: 12px; right: 0; top: 0; animation-delay: 0s; }
.z2 { font-size: 15px; right: 12px; top: -10px; animation-delay: 0.6s; }
.z3 { font-size: 20px; right: 22px; top: -22px; animation-delay: 1.2s; }

@keyframes zFloat {
  0% { opacity: 0; transform: translateY(0) translateX(0) scale(0.6); }
  20% { opacity: 0.7; }
  50% { opacity: 0.7; transform: translateY(-12px) translateX(6px) scale(1); }
  80% { opacity: 0.3; }
  100% { opacity: 0; transform: translateY(-28px) translateX(14px) scale(0.4); }
}

/* ── Text ── */
.splash-text {
  position: relative;
  display: flex;
  align-items: baseline;
  gap: 0;
}

.splash-title {
  font-size: 1.6rem;
  font-weight: 700;
  letter-spacing: 0.14em;
  color: rgba(220, 215, 245, 0.9);
  font-family: 'Inter', system-ui, sans-serif;
  text-shadow: 0 0 30px rgba(124,111,255,0.2);
}

.splash-dots {
  display: inline-flex;
  margin-left: 2px;
}

.dot {
  font-size: 1.6rem;
  font-weight: 700;
  color: rgba(220, 215, 245, 0.9);
  animation: dotPulse 1.4s ease-in-out infinite;
}
.dot:nth-child(2) { animation-delay: 0.2s; }
.dot:nth-child(3) { animation-delay: 0.4s; }

@keyframes dotPulse {
  0%, 60%, 100% { opacity: 0.15; }
  30% { opacity: 1; }
}

.splash-sub {
  font-size: 0.78rem;
  color: rgba(180, 190, 220, 0.5);
  letter-spacing: 0.06em;
  font-weight: 500;
  margin-top: -0.5rem;
  animation: subFade 3s ease-in-out infinite;
}

@keyframes subFade {
  0%, 100% { opacity: 0.35; }
  50% { opacity: 0.65; }
}
</style>
