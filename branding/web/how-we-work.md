# How we deliver

Software projects fail when people mistake activity for progress. Writing more tickets, generating more code, and adding more developers doesn't help if nobody agrees on what the system is actually supposed to do.

Our method connects the dots between discovering the rules, making decisions, writing the code, and proving it works.

## Human judgment is the real value

Software delivery isn't a race to write the most lines of code. It's about deciding which problems are worth solving, understanding the constraints, making smart trade-offs, and deciding when the result is good enough to launch.

We build reflection into our delivery rhythm. Our review points give your team the time to challenge assumptions and think through consequences *before* we start building. This isn't red tape; it's the most important part of the job.

## Start with evidence, not guesses

We gather information from everywhere: the code, the docs, the tests, the live servers, and the people who use the system. 

We don't mash all this information together. We keep the sources separate so we can see exactly where they agree and where they conflict. A rule found in a legal document is much more reliable than a rule we guessed by reading old code.

## Keep uncertainty visible

Not knowing something is a valid finding. Hiding unknowns just to make a project plan look good is dangerous. 

When we find a gap or a conflict, we ask the right person on your team to make a decision. If we can't resolve it, we document it as a known risk. We don't let AI guess the answer.

## Deliver in safe, bounded waves

We break massive projects down into phases that can be reviewed and accepted on their own. Each phase has:
- A clear goal.
- Known evidence and requirements.
- Visible risks and dependencies.
- Clear tests for success.
- A finished product that becomes the foundation for the next phase.

We base these phases on the actual architecture of the system, not just what fits into a two-week sprint.

## Review before we build

Before we write any code, we have two major review points:
1. **The Plan:** We review the proposed boundaries of the project to make sure we're solving the right problem.
2. **The Specs:** We review the exact requirements and evidence to make sure we haven't missed anything.

We only start building when everyone agrees on the plan. 

## AI helps, but humans are accountable

We use AI to research code, write tests, and speed up development. But AI doesn't decide what your business rules should be, it doesn't resolve arguments, and it doesn't sign off on a launch.

Propellerhead is accountable for the delivery. Your team is accountable for the business goals. AI is just a tool we use to get there faster.

## Test the code, not the coder's confidence

Testing is only useful if it's independent. If a developer writes a test just to prove their own code works, that's helpful, but it's not enough for a critical system.

We use a mix of automated tests, security scans, engineering reviews, and operational checks. We are honest about what we've proven and what we haven't. If a critical feature doesn't have an independent test, we flag it as a risk.

## Preserve a living baseline

When a phase is finished, the specs, the decisions, and the tests live on with the product. The next time you need to change the system, you start from this clear baseline instead of digging through old emails and Jira tickets.

## Keep your infrastructure options open

When we build applications using our Omnia runtime, your business logic is compiled into WebAssembly. It runs completely separate from your infrastructure (like databases, messaging, or identity services).

This means you can swap out your cloud provider or database later without rewriting your core application. It keeps your options open and prevents vendor lock-in.

[Explore infrastructure portability](infrastructure-portability.md)

## You stay in control

You own your code, your specs, your evidence, and your deployment. We integrate with your existing security and release processes.

We want you to stay with us because our process makes your life easier, not because you're technically locked in and can't leave.

## The practical rhythm

1. **Plan:** We survey the system and propose the phases.
2. **Review:** You confirm we're tackling the right problems.
3. **Refine:** We extract the evidence, expose the gaps, and write the specs.
4. **Review again:** You confirm exactly what will be built.
5. **Execute:** We build, test, and review the code.
6. **Accept:** We publish the result through your normal controls, and it becomes the baseline for the next phase.

[Explore modernization](modernization.md)  
[Explore continuous assurance](continuous-assurance.md)
