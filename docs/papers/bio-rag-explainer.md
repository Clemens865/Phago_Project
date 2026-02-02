# How a Digital Immune System Learns to Answer Questions Better Over Time

**An accessible introduction to biologically-inspired retrieval-augmented generation**

---

## 1. The Problem: Search Engines Forget You the Moment You Leave

Imagine walking into a library every morning and asking the librarian the same kind of question: "I need something about climate policy in Southeast Asia." The first day, she disappears into the stacks and comes back twenty minutes later with a decent book. The second day, you ask something similar, and she starts from scratch. She has no memory of yesterday. No notes. No shortcuts. Every visit is day one.

This is how most search and retrieval systems work today. You type a question into a search engine or a chatbot, it scans its database, returns some results, and then forgets everything about the interaction. The next time you -- or anyone else -- asks a related question, the system does exactly the same amount of work and makes exactly the same mistakes. It never learns which answers were helpful. It never notices that two topics tend to come up together. It treats every query as if it has never seen a question before.

This is a serious limitation. Humans do not work this way. A good librarian remembers that the person who asks about climate policy also tends to ask about rice farming and water management. Over time, she builds a mental map: these topics are neighbors. She starts pulling related books before you even ask. She gets faster. She gets better. She learns.

The question we set out to answer is simple: can we build a retrieval system that does the same thing? Can we make a digital librarian that gets better at answering your questions just by being asked?

---

## 2. How Cells Learn: Neurons That Fire Together Wire Together

To solve this problem, we looked at how biological systems learn. Not classrooms or textbooks -- actual cells.

In your brain, learning happens through a principle that neuroscientists sometimes summarize as "neurons that fire together wire together." When two nerve cells are activated at the same time, the connection between them grows stronger. The next time one of them fires, the other is more likely to fire too. This is how you form associations. You hear a song, and it reminds you of a summer vacation, because those two memories were activated together enough times that the pathway between them became well-worn.

Your immune system does something similar. When your body encounters a pathogen -- a virus, a bacterium -- certain immune cells respond. If that response works (the pathogen is destroyed), those cells are reinforced. They multiply. They stick around. The next time the same pathogen appears, the response is faster and stronger. This is why you rarely get the same cold twice. Your immune system remembered what worked and made that pathway easier to activate.

Both of these systems share a key property: they improve through use. They do not need a teacher to come in and reprogram them. They do not need to be taken offline and retrained. They simply get better at the things they do most often, in real time, through reinforcement.

We borrowed this idea.

---

## 3. Building a Digital Brain That Learns from Questions

Here is how the system works, explained through our library metaphor.

Picture a library, but instead of shelves arranged in rows, imagine every book connected to every other book by a thread. Some threads are thick and strong; others are thin and barely visible. The thickness of each thread represents how closely related two books are. A book about ocean currents might have a thick thread connecting it to a book about marine biology, and a thinner one connecting it to a book about shipping logistics.

Now picture the librarian. Every time someone asks a question, the librarian walks through the library following threads. She picks up the books she finds along the strongest paths. But here is the key difference from a normal library: after she delivers the books, she watches what happens. Did the reader find what they needed? Did they pick up the first book or skip to the third? Did they come back with a follow-up question?

Based on what she observes, the librarian adjusts the threads. If the first book she recommended was exactly right, the threads she followed to reach it get a little thicker. If a book was irrelevant, the threads leading to it get a little thinner. Over time, the map of threads -- the graph -- reshapes itself around the questions people actually ask.

In technical terms, the system maintains a knowledge graph where documents are nodes and the edges between them carry weights. When a query arrives, the system retrieves documents not just by matching keywords or vector similarity, but by walking the graph along weighted edges. After retrieval, the system receives a signal about how useful the results were -- did the top-ranked document contain the answer? -- and uses that signal to strengthen or weaken edges. Strong performers get reinforced. Weak ones fade.

If you were to draw this as a diagram, you would see clusters forming over time. Documents that are frequently useful together drift closer in the graph. Islands of related knowledge emerge, connected by bridges to other islands. A question about immune cells might connect to documents about cell signaling, which connect to documents about network theory, which connect back to documents about search algorithms. The graph becomes a living map of how knowledge relates to knowledge, shaped entirely by the questions people ask.

No one programs these connections. They emerge.

---

## 4. The Experiment: 40 Documents, 20 Questions, 10 Rounds

We tested this with a straightforward experiment. We prepared a knowledge base of 40 documents spanning 4 topics and asked the system 20 questions. Then we asked them again. And again. Ten rounds total -- the same 20 questions, asked 10 times each.

We measured performance using two metrics. Mean Reciprocal Rank (MRR) tells us how close the first relevant result is to the top: if the correct answer is the very first result, that scores 1.0. If second, it scores 0.5. Third, 0.33. Precision at 5 (P@5) tells us what fraction of the top 5 results are actually relevant.

The graph-based system achieved an MRR of 0.714 and a P@5 of 0.270. For comparison, a traditional TF-IDF baseline achieved a P@5 of 0.658. What this tells us is nuanced: the graph retrieval finds the first relevant result faster on average (higher MRR), but has lower overall precision than keyword-based retrieval. The graph is better at surfacing *one* highly relevant document quickly, but TF-IDF is better at filling the top-5 list with multiple relevant documents.

Importantly, we did not observe round-over-round improvement in this experiment. The graph's performance remained relatively stable across all 10 rounds, suggesting that the reinforcement mechanism needs more diverse query patterns or a longer timescale to demonstrate cumulative learning. The system worked as designed -- it retrieved documents by walking weighted edges -- but the hypothesized improvement over time did not materialize in this particular test.

No model was retrained. No weights in a neural network were updated. No new data was added. The only thing that changed was the structure of the graph, reshaped by the simple signal of which answers were good and which were not.

---

## 5. What This Means: Self-Improving Retrieval Without Retraining

Most AI systems today improve through a painful and expensive process: you collect new data, you retrain the model (which can take days and cost thousands of dollars in compute), you deploy the new version, and you hope it performs better. If it does not, you start over.

The system described here sidesteps that entire process. It improves continuously, in real time, through use. Every question makes it a little bit smarter. Every answer it gets right reinforces the pathways that led to that answer. Every answer it gets wrong weakens the pathways that led astray.

This has practical implications. A customer support system built this way would get better at answering your company's specific questions over time, without anyone needing to retrain anything. A research assistant would learn which papers are most relevant to your field. A medical knowledge base would learn which connections between symptoms and conditions matter most for the clinicians who use it.

The system is also transparent in a way that many AI systems are not. Because the learning happens in a graph -- a structure you can visualize and inspect -- you can see why the system is making the recommendations it makes. You can trace the strengthened edges. You can ask: why did it rank this document first? And the answer is visible in the graph: because the last five times someone asked a question like yours, this document was helpful, and the threads leading to it grew stronger.

The key takeaway is this: **the system gets better at answering your questions just by being asked.** Like a librarian who remembers which shelves you visit, like an immune system that remembers which responses worked, this digital system builds stronger paths between the knowledge that matters most -- automatically, continuously, and without retraining.

The library remembers you. And it rearranges its shelves while you sleep.
