# Build new systems right the first time

AI tools can turn an idea into working code incredibly fast. But speed can hide the decisions you'll have to live with for years: what happens when the system fails, how you'll test it, and which cloud provider you're accidentally locking yourself into.

Propellerhead builds new critical systems with the same discipline we use to fix old ones. We make sure the goals are clear, the decisions are deliberate, and the infrastructure is flexible.

**Primary action:** Discuss a new critical system

## Code isn't the first decision

Before the rush to write code locks you into a path, we sit down with the people accountable for the system to figure out:

- What is the actual goal, and who does this affect?
- What are the hard constraints and security rules?
- What happens if the system fails?
- How will we know it's good enough to launch?
- What infrastructure do we need, and how do we avoid getting locked into one vendor?

We aren't asking you to write a 500-page spec upfront. We just want to identify the decisions that are too expensive to get wrong.

## Thinking is productive work

Software is full of judgments about risk, users, and the future. A good process gives you time to look at those judgments before they become permanent.

Our review points are working sessions, not red tape. Your team and our engineers look at the plans, challenge assumptions, and make the hard decisions *before* we start writing code. 

The goal isn't to be 100% certain about everything. It's to define the smallest, safest piece of work we can build, test, and learn from.

## AI changes how we work, not who is responsible

Open-source AI tools are making it cheaper and easier to write code. We think that's great. 

But as writing code gets cheaper, the real value of software engineering changes. The hard part is now:
- Picking the right problem to solve.
- Understanding the consequences of a design.
- Testing the results independently.
- Taking responsibility for what goes live.
- Building it so the next team can easily understand and change it.

We use AI to research, write, and test faster. But we never use AI to replace human judgment or accountability.

## Build in safe steps

We build in clear, bounded phases. Each phase establishes:
- What we are building and why.
- The decisions and evidence supporting it.
- What we still don't know.
- The infrastructure we need.
- How we will test and accept the work.

The finished phase becomes the solid foundation for the next one. Your system accumulates knowledge, not just code.

## Keep your options open with Omnia

When we use our Omnia runtime, your business logic is compiled into WebAssembly. It runs completely separate from your infrastructure (like databases, messaging, or identity services).

This means you can swap out your cloud provider or database later without rewriting your core application. 

It doesn't make moving clouds effortless—you still have to move data and test the new setup—but it makes it much less painful. You aren't held hostage by today's infrastructure choices.

[Explore infrastructure portability](infrastructure-portability.md)

## Is this for you?

This approach is best when a new system:
- Will be critical to your business or the public.
- Needs to be maintained and understood for years.
- Has strict security or regulatory rules.
- Needs to run on your own infrastructure, or you want the freedom to change clouds later.
- Will be handed off to different teams over time.
- Would be a disaster if it was built wrong.

If you just need a quick prototype or a simple internal tool, you don't need this level of rigor. We are built for the systems you can't afford to get wrong.

## Start with the goals, not the code

Bring us the outcome you want, the constraints you know about, and the decisions that are keeping you up at night. We'll help you figure out the safest first step.

[Start a conversation](contact.md)
