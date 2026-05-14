# AI Memory in Five Scenes — TerranSoul, Explained for Everyone

> **Inspired by** cognee's [*AI memory in five scenes*](https://www.cognee.ai/blog/fundamentals/ai-memory-in-five-scenes)
> (Vasilije Markovic et al., 2025). The cognee article is the single
> clearest plain-English explanation we have read of how "AI memory"
> grew from a chatty stranger into something that can actually help you
> think. We borrowed only the *shape* of their five scenes — none of
> their prose, examples, or images — to tell the same story about
> TerranSoul. Credit and licensing notes live in
> [`CREDITS.md`](../CREDITS.md).
>
> If you are a developer and want the version with file paths and
> commands, read the sibling document
> [`ai-memory-five-scenes-terransoul.md`](ai-memory-five-scenes-terransoul.md)
> instead. This one is for everyone else.

So you've heard that TerranSoul is "an AI companion with memory." Fine
words. But what does *memory* actually mean for an AI? Why does it
matter? And what is your TerranSoul companion really doing differently
from the chatbot in your browser tab?

The easiest way to answer that is to walk through five everyday scenes.
Each one shows a different kind of "remembering." By Scene 5 you will
know exactly which kind TerranSoul is, and why we built it that way.

---

## Scene 1: A kid in a library

Imagine you are twelve years old and you walk into a real library —
shelves, dust, the smell of old paper — looking for help with a school
project on dinosaurs. There are three people behind the desk.

The **first person** is bright and chatty. He can talk about dinosaurs
all afternoon. He just started this week, though, so he has no idea
where anything is in *this* library. Lots of knowledge, no sense of
place.

The **second person** stares at a computer. Tell him a book title and
he will tell you the shelf number and read the back cover out loud. He
does not know whether the book is any good for your project. All search,
no judgment.

The **third person** is the senior librarian. She has worked here for
twenty years. She does not just *find* books — she remembers that the
chapter you actually need is buried inside an old history book three
shelves over, and she knows it connects to the picture book your
classmate borrowed last week. She gives you the answer, not just the
search results.

Most AI products today are one of those three people:

- **The first person** is a plain chatbot. Smart, articulate, knows
  nothing about *you*.
- **The second person** is a basic search-and-quote assistant.
  Accurate, but it cannot tell what is useful.
- **The third person** is what TerranSoul is trying to be.

TerranSoul lets you choose which "brain" your companion uses — a free
cloud one, a paid cloud one, or one that runs entirely on your own
computer with no internet. The important part is that all three of
those brains read from the **same library**: the memory TerranSoul has
quietly been building about you. Switch the brain, keep the librarian.

---

## Scene 2: A high schooler picking a movie

It's Friday night. You open a streaming app and want a recommendation.
There are three little assistants you could ask.

The **first one** is the chatbot in your phone. It cheerfully suggests
a beloved animated film — but the film is on a competing service you do
not subscribe to. It does not know what is on *your* shelf, or that you
already watched it last month, or that you are sharing the couch with a
seven-year-old.

The **second one** is the streaming service's built-in bot. It searches
the catalogue and dumps a wall of plot summaries, cast lists, and star
ratings at you. There are real answers in there somewhere, but you have
to dig for them.

The **third one** is a friend who has watched movies with you for
years. Before answering, they remember: *you like Pixar, you have a kid
on the couch, you fell asleep in the last quiet drama you tried, and
you cannot stand sad endings on a Friday.* They give you two short
suggestions and a one-sentence reason for each.

TerranSoul is built to be that friend. Every conversation it has with
you is shaped by:

- **What you've told it about yourself** (your name, your hobbies, your
  pet peeves).
- **What it has watched you do** (which projects you keep returning to,
  which topics keep coming up, which suggestions you accepted and which
  you ignored).
- **How recent things are** (a preference you mentioned an hour ago
  outweighs one from two years ago).
- **What kind of question you just asked** ("what did I do?" is
  answered differently than "what is true?" or "how do I?").
- **What you said is private** (some memories never leave your
  computer, some sync between *your* devices, some you choose to share
  — and the most-private rule always wins).

The result is that TerranSoul does not shout your whole memory at the
AI on every message. It hands the AI a small, tidy briefing tailored to
*this* moment. That is why the answers feel personal instead of
generic.

---

## Scene 3: A college student cramming for an exam

You are a few days from a final. You have lecture notes on your laptop,
PDFs of slides, scanned chapters from the textbook, and a pile of your
own scribbles. You ask a chatbot, *"What did Professor Miller say
about entropy?"*

A plain chatbot will give you a textbook definition of entropy. Lovely
— but Professor Miller is not in there. You could paste your notes in
one document at a time, but there are too many, and the chatbot starts
forgetting the early ones.

The trick that actually works is this: a tool quietly chops your notes
into bite-sized pieces, gives each piece a kind of "fingerprint" that
captures its meaning, and stores them. When you ask a question, the
tool fingerprints the *question* the same way and pulls back the few
pieces that look most similar. Those pieces get handed to the AI along
with your question, and now the AI is answering from your actual
notes.

That trick has a name in the industry — but you do not need it. What
matters is that **TerranSoul does this for everything you let it
remember**: chats, documents you drop in, screenshots, voice notes,
files in folders you point it at. It pulls back the right handful of
pieces, ranks them, throws out the noisy ones, and only then asks the
AI to answer.

There are two honest things to say about this kind of memory:

1. It is genuinely good at *"what did so-and-so say about X."* That is
   the bread and butter.
2. It is not enough by itself. Ask *"which examples did the professor
   use that were not in the textbook?"* and a fingerprint match alone
   gets confused, because that question is about *relationships*
   between things, not just similarity. Hold that thought for Scene 4.

TerranSoul also keeps a strict budget on how much of your notes it
hands to the AI in any one message. Otherwise the AI gets buried, slows
down, and the answer gets worse, not better. Less is more.

---

## Scene 4: A junior engineer hunting for a job

You are looking for your next job. Not just any job — you want
something specific:

- *Remote-first.*
- *A startup with 20 to 50 people.*
- *Already raised a Series B.*
- *Uses Python and React.*
- *Not in fintech.*

You ask a chatbot. It hands you a friendly-looking list. Half the
companies are too big. Two are in fintech. One is "remote" but actually
means "remote two days a week from our Berlin office." Useless.

You try the fingerprint trick from Scene 3. Better — at least the
postings now mention your keywords. But "startup culture" sneaks in for
a 600-person company. "Remote" sneaks in for a hybrid role. Your
*not in fintech* rule gets quietly ignored.

The real fix is to teach the system to *name things*. Not just "this
paragraph mentions Python" but **"this is a company, that is a job,
this is a skill, that is a funding stage, those are connected like
this."** Once the system has named the pieces and drawn the lines
between them, your messy wish list becomes a precise question:

> *Find me jobs whose company is at Series B, whose team size is
> between 20 and 50, whose required skills include Python and React,
> whose industry is not fintech, and whose location policy is
> remote-first — and show me the exact sentence you got each fact
> from.*

That is the leap from "find similar text" to "answer the question."
TerranSoul does this leap. Behind the scenes it keeps a quiet map of
the people, places, projects, decisions, and preferences in your life,
with little arrows showing how they connect — *you decided X on date Y
because of person Z*. When you ask a question that needs *relationships*
to answer, TerranSoul walks the map.

Two things follow from that map being there:

- **TerranSoul can hold contradictions.** If something you said three
  months ago no longer matches what you said yesterday, both versions
  are kept, the older one fades, and the new one wins. Nothing is
  silently overwritten and nothing is silently lost.
- **TerranSoul can show its receipts.** When it tells you something, it
  can point back at the original message, file, or moment it learned it
  from. You never have to take its word for it.

---

## Scene 5: The veteran at a party

You are at a party and someone asks what you do. You say you work on
AI memory. Most people change the subject. One person leans in. They
have built this stuff before. They already know that the impressive
demo and the system that survives a Tuesday afternoon are two different
things.

The two of you end up agreeing on the same short list — the things
that *actually* decide whether an AI memory is useful in real life:

**It has to be fast.** Beautiful retrieval that takes thirty seconds is
worse than a mediocre answer in one second, because nobody waits.
TerranSoul keeps things snappy by remembering popular answers, by
splitting its memory into smaller pieces it can search in parallel, by
skipping the heavy machinery for trivial questions like *"hi,"* and by
falling back gracefully on slower hardware (your phone) instead of
freezing.

**It has to be honest about quality.** A memory that confidently makes
things up is worse than no memory. TerranSoul keeps every memory with a
*confidence* attached, lets old facts decay if newer ones contradict
them, and treats different kinds of knowledge differently — your
casual notes can move fast and loose, but your legal, financial, and
shared-team facts move carefully and require agreement before they
change.

**It has to be measurable.** "Trust us, it's good" is not a feature.
TerranSoul ships with public benchmarks against the same long-memory
test sets the research community uses, and reports both its wins and
its ties honestly. When a new trick helps on one kind of question and
hurts on another, we turn it on only for the kind it helps. We did not
guess that — we measured it.

This is the scene TerranSoul is built for. Not the demo. The Tuesday.

---

## So what is TerranSoul, in one breath?

It is the senior librarian from Scene 1, who happens to be your
streaming-night friend from Scene 2, who has read every note you ever
gave it like the study tool from Scene 3, who keeps a quiet relationship
map like the job-hunt assistant from Scene 4 — and who has been built
to survive the Tuesday-afternoon problems from Scene 5.

In plainer words: **TerranSoul remembers your life the way a thoughtful
friend would — privately, accurately, and with receipts — and lets you
choose which AI brain gets to talk to that memory.**

That is the whole pitch. Everything else in the documentation is the
craft underneath it.

---

## Want to go deeper?

- **For developers and curious readers** — the same five scenes mapped
  to the actual files and subsystems in the codebase:
  [`ai-memory-five-scenes-terransoul.md`](ai-memory-five-scenes-terransoul.md).
- **For the original story** — cognee's [*AI memory in five
  scenes*](https://www.cognee.ai/blog/fundamentals/ai-memory-in-five-scenes),
  whose framing inspired both of these docs.
- **For everything we have learned from other people** — the
  [`CREDITS.md`](../CREDITS.md) page, where every blog post, paper, and
  open-source project that shaped TerranSoul gets a thank-you.
