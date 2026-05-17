---
id: enumerate
description: Per-language enumeration brief for /change:survey (C#).
language: csharp
---

# C# surface enumeration for `/change:survey`

This brief drives the LLM enumeration step of `/change:survey` for C# / .NET source roots. The skill resolves this file from the detected language and prompts the model to produce a candidate `surfaces.json` for one source. The CLI validates, canonicalizes, and writes the canonical artifact; this brief carries every C#-specific decision the model has to make.

The C# ecosystem ships several competing idioms for the same kind (controllers vs minimal API; MassTransit vs MediatR; Hangfire vs Quartz). Enumerate the surfaces actually present in the source — never the surfaces a framework theoretically supports. If a project does not import a framework's package, do not emit surfaces for it.

---

## Scope

Frameworks covered in v1:

- **ASP.NET Core 6+ controllers** — each `[HttpGet/Post/Put/Patch/Delete/Head/Options]` (or `[AcceptVerbs]`) action method on a class derived from `ControllerBase` / `Controller` is one `http-route`.
- **ASP.NET Core minimal API** — each top-level `app.MapGet/MapPost/MapPut/MapPatch/MapDelete/MapMethods(…)` registration is one `http-route`.
- **ASP.NET Core endpoint routing with conventional `[Route]` attributes** — controllers that rely on `[Route]` (with or without `[ApiController]`) for path templating; the action verb attributes still bind the surface.
- **MassTransit** — each `IConsumer<TMessage>` / `IConsumer<Batch<TMessage>>` implementation is one `message-sub`; each `IPublishEndpoint.Publish(...)` / `ISendEndpoint.Send(...)` call site is one `message-pub`.
- **MediatR** — each `INotificationHandler<TNotification>` implementation is one `message-sub`. (Request handlers — `IRequestHandler<TRequest, TResponse>` — are in-process RPC, not a surface; do not emit them.)
- **Hangfire** — each `RecurringJob.AddOrUpdate(...)` / `RecurringJob.AddOrUpdate<TService>(...)` registration is one `scheduled-job`.
- **Quartz.NET** — each `IJob` implementation paired with a registered cron / interval trigger is one `scheduled-job`.
- **System.CommandLine** — each `Command` / `RootCommand` with a bound handler is one `cli-command`.
- **Typed `HttpClient` via `IHttpClientFactory` named clients** — each named client created with `factory.CreateClient("name")` and used at a call site is one `external-call-out`.

`ws-handler` and `ui-route` are not in v1 scope for C#: SignalR hubs and Razor / Blazor pages would belong here when the brief grows. If the source contains them, leave them out and let `propose` rely on operator review.

## Schema

The candidate document is a `surfaces.json` matching the schema verbatim. Every emitted surface MUST use one of the closed `kind` values:

- `http-route`
- `message-pub`
- `message-sub`
- `ws-handler`
- `scheduled-job`
- `cli-command`
- `ui-route`
- `external-call-out`

Unknown kinds fail validation. Each `Surface` object has the field set:

- `id` (string, ≥1 char) — stable identifier unique within this `surfaces.json`. Reruns must diff cleanly: the brief produces it from `kind` + the legacy identifier, kebab-cased (e.g. `http-post-api-users-register`, `message-sub-user-created`, `scheduled-job-daily-invoices`).
- `kind` (string, closed enum above).
- `identifier` (string, ≥1 char) — legacy spelling of the surface (route, message type, command name, schedule name, outbound call).
- `handler` (string, ≥1 char) — handler or call-site reference, typically `file:function`.
- `touches` (array of strings, ≥1 char each) — source files reached from the handler, sorted alphabetically, relative to the source root.
- `declared-at` (array of strings, ≥1 entry, ≥1 char each) — declaration sites where the surface is registered with its framework, sorted alphabetically. Relative paths optionally suffixed with `:<line>`.

Path-under-source-root rule (applies to every entry in `touches[]` and `declared-at[]`):

- Relative paths only — no leading `/`, no Windows drive letter (`C:\…`, `\\?\…`).
- No `..` segments.
- The path must resolve to a file under the source root when joined.
- Skip vendored / build / IDE roots: `bin/`, `obj/`, `packages/`, `.vs/`, plus any vendored directory checked into the source root.

The CLI re-checks every rule before writing; emitting a forbidden path triggers `surfaces-touches-out-of-tree` and re-enters the repair loop.

## Worked examples

One worked example per applicable kind. Each shows a short C# input snippet, the framework signature that fired, and the expected `Surface` JSON block that the brief should produce.

### `http-route` — controller (`[ApiController]` + `[HttpPost("…")]`)

Signature: class derived from `ControllerBase`, attributed with `[ApiController]` and (optionally) a class-level `[Route("…")]`; each action method carries an `[HttpGet/Post/…("…")]` attribute.

Input — `Controllers/UsersController.cs`:

```csharp
namespace Acme.Api.Controllers;

[ApiController]
[Route("api/users")]
public class UsersController : ControllerBase
{
    private readonly IUserRegistrationService _registrations;

    public UsersController(IUserRegistrationService registrations) => _registrations = registrations;

    [HttpPost("register")]
    public async Task<IActionResult> RegisterUser(RegisterRequest request, CancellationToken ct)
    {
        var user = await _registrations.Register(request, ct);
        return CreatedAtAction(nameof(RegisterUser), new { id = user.Id }, user);
    }
}
```

Surface:

```json
{
  "id": "http-post-api-users-register",
  "kind": "http-route",
  "identifier": "POST /api/users/register",
  "handler": "Controllers/UsersController.cs:RegisterUser",
  "touches": [
    "Controllers/UsersController.cs",
    "Models/RegisterRequest.cs",
    "Services/IUserRegistrationService.cs",
    "Services/UserRegistrationService.cs"
  ],
  "declared-at": ["Controllers/UsersController.cs:11"]
}
```

### `http-route` — minimal API (`app.MapGet("…", …)`)

Signature: `WebApplication app = builder.Build();` followed by one or more `app.MapGet/MapPost/MapPut/MapPatch/MapDelete/MapMethods(...)` calls registering inline or referenced handlers.

Input — `Program.cs`:

```csharp
var builder = WebApplication.CreateBuilder(args);
builder.Services.AddSingleton<IUserStore, UserStore>();
var app = builder.Build();

app.MapGet("/users/{id}", (int id, IUserStore store) => store.Find(id));

app.Run();
```

Surface:

```json
{
  "id": "http-get-users-id",
  "kind": "http-route",
  "identifier": "GET /users/{id}",
  "handler": "Program.cs:get-users-id",
  "touches": [
    "Program.cs",
    "Stores/IUserStore.cs",
    "Stores/UserStore.cs"
  ],
  "declared-at": ["Program.cs:5"]
}
```

### `http-route` — endpoint routing with conventional `[Route]`

Signature: controller attributed with class-level `[Route("…")]` (templated or not), no `[ApiController]` required; action methods carry verb attributes (`[HttpGet]`, `[HttpPost]`, …) whose route template composes with the class-level template.

Input — `Controllers/HomeController.cs`:

```csharp
[Route("[controller]")]
public class HomeController : Controller
{
    [HttpGet("about")]
    public IActionResult About() => View();
}
```

Surface:

```json
{
  "id": "http-get-home-about",
  "kind": "http-route",
  "identifier": "GET /home/about",
  "handler": "Controllers/HomeController.cs:About",
  "touches": [
    "Controllers/HomeController.cs",
    "Views/Home/About.cshtml"
  ],
  "declared-at": ["Controllers/HomeController.cs:1"]
}
```

### `message-pub` — MassTransit publish (`IPublishEndpoint.Publish(...)`)

Signature: a constructor-injected `IPublishEndpoint` (or `ISendEndpoint`) with a call site `await _bus.Publish(new TMessage(...))`. Identifier is the message type name.

Input — `Services/OrderService.cs`:

```csharp
public class OrderService
{
    private readonly IPublishEndpoint _bus;

    public OrderService(IPublishEndpoint bus) => _bus = bus;

    public async Task PlaceOrder(Order order, CancellationToken ct)
    {
        await _bus.Publish(new OrderPlaced(order.Id), ct);
    }
}
```

Surface:

```json
{
  "id": "message-pub-order-placed",
  "kind": "message-pub",
  "identifier": "OrderPlaced",
  "handler": "Services/OrderService.cs:PlaceOrder",
  "touches": [
    "Contracts/OrderPlaced.cs",
    "Models/Order.cs",
    "Services/OrderService.cs"
  ],
  "declared-at": ["Services/OrderService.cs:9"]
}
```

### `message-sub` — MassTransit consumer (`IConsumer<TMessage>`)

Signature: a class implementing `IConsumer<TMessage>` (or `IConsumer<Batch<TMessage>>`) with a `Consume(ConsumeContext<TMessage> context)` method. Identifier is the message type name.

Input — `Consumers/UserCreatedConsumer.cs`:

```csharp
public class UserCreatedConsumer : IConsumer<UserCreated>
{
    private readonly IWelcomeMailer _mailer;

    public UserCreatedConsumer(IWelcomeMailer mailer) => _mailer = mailer;

    public async Task Consume(ConsumeContext<UserCreated> context)
    {
        await _mailer.SendWelcome(context.Message.UserId);
    }
}
```

Surface:

```json
{
  "id": "message-sub-user-created",
  "kind": "message-sub",
  "identifier": "UserCreated",
  "handler": "Consumers/UserCreatedConsumer.cs:Consume",
  "touches": [
    "Consumers/UserCreatedConsumer.cs",
    "Contracts/UserCreated.cs",
    "Services/IWelcomeMailer.cs",
    "Services/WelcomeMailer.cs"
  ],
  "declared-at": ["Consumers/UserCreatedConsumer.cs:1"]
}
```

### `message-sub` — MediatR notification handler (`INotificationHandler<T>`)

Signature: a class implementing `INotificationHandler<TNotification>` with a `Handle(TNotification notification, CancellationToken ct)` method. Identifier is the notification type name. (`IRequestHandler<TRequest, TResponse>` is in-process RPC and is NOT a surface — see Anti-patterns.)

Input — `Handlers/UserCreatedHandler.cs`:

```csharp
public class UserCreatedHandler : INotificationHandler<UserCreatedNotification>
{
    private readonly IAuditLog _audit;

    public UserCreatedHandler(IAuditLog audit) => _audit = audit;

    public Task Handle(UserCreatedNotification notification, CancellationToken ct) =>
        _audit.Record($"user.created:{notification.UserId}", ct);
}
```

Surface:

```json
{
  "id": "message-sub-user-created-notification",
  "kind": "message-sub",
  "identifier": "UserCreatedNotification",
  "handler": "Handlers/UserCreatedHandler.cs:Handle",
  "touches": [
    "Handlers/UserCreatedHandler.cs",
    "Notifications/UserCreatedNotification.cs",
    "Services/AuditLog.cs",
    "Services/IAuditLog.cs"
  ],
  "declared-at": ["Handlers/UserCreatedHandler.cs:1"]
}
```

### `scheduled-job` — Hangfire (`RecurringJob.AddOrUpdate(...)`)

Signature: a `RecurringJob.AddOrUpdate(...)` or `RecurringJob.AddOrUpdate<TService>(...)` call registering a job by id with a cron expression. Identifier is the job id.

Input — `Program.cs`:

```csharp
RecurringJob.AddOrUpdate<IInvoiceJob>("daily-invoices", j => j.Run(CancellationToken.None), Cron.Daily);
```

Surface:

```json
{
  "id": "scheduled-job-daily-invoices",
  "kind": "scheduled-job",
  "identifier": "daily-invoices",
  "handler": "Jobs/InvoiceJob.cs:Run",
  "touches": [
    "Jobs/IInvoiceJob.cs",
    "Jobs/InvoiceJob.cs",
    "Repositories/InvoiceRepository.cs"
  ],
  "declared-at": ["Program.cs:34"]
}
```

### `scheduled-job` — Quartz (`IJob` + trigger config)

Signature: a class implementing `IJob` with `Execute(IJobExecutionContext context)`, paired with a registered trigger (cron / simple) carrying an explicit identity. Identifier is the trigger / job identity.

Input — `Jobs/HouseKeepingJob.cs` + `Program.cs`:

```csharp
public class HouseKeepingJob : IJob
{
    private readonly IPurgeService _purge;

    public HouseKeepingJob(IPurgeService purge) => _purge = purge;

    public Task Execute(IJobExecutionContext context) => _purge.PurgeStale(context.CancellationToken);
}

// Program.cs
quartz.ScheduleJob<HouseKeepingJob>(
    t => t.WithIdentity("housekeeping").WithCronSchedule("0 0 3 * * ?"));
```

Surface:

```json
{
  "id": "scheduled-job-housekeeping",
  "kind": "scheduled-job",
  "identifier": "housekeeping",
  "handler": "Jobs/HouseKeepingJob.cs:Execute",
  "touches": [
    "Jobs/HouseKeepingJob.cs",
    "Services/IPurgeService.cs",
    "Services/PurgeService.cs"
  ],
  "declared-at": ["Jobs/HouseKeepingJob.cs:1", "Program.cs:18"]
}
```

### `cli-command` — System.CommandLine (`Command` / `RootCommand`)

Signature: a `Command` (or `RootCommand`) constructed with a name and a bound handler via `SetHandler` or a `CommandHandler.Create` delegate. Identifier is the command name path (root command excluded; subcommands joined with spaces, e.g. `users import`).

Input — `Program.cs`:

```csharp
var importCommand = new Command("import", "Import users from CSV");
importCommand.SetHandler(ImportUsers);
var root = new RootCommand("Acme CLI") { importCommand };
return await root.InvokeAsync(args);
```

Surface:

```json
{
  "id": "cli-command-import",
  "kind": "cli-command",
  "identifier": "import",
  "handler": "Program.cs:ImportUsers",
  "touches": [
    "Program.cs",
    "Services/IUserImportService.cs",
    "Services/UserImportService.cs"
  ],
  "declared-at": ["Program.cs:3"]
}
```

### `external-call-out` — typed `HttpClient` via `IHttpClientFactory` named client

Signature: an injected `IHttpClientFactory` followed by a `factory.CreateClient("name")` call and a downstream `client.GetAsync/PostAsync/SendAsync(...)` call site. Identifier is the named-client label plus the method and path template (e.g. `billing GET /invoices/{id}`).

Input — `Gateways/BillingGateway.cs`:

```csharp
public class BillingGateway
{
    private readonly IHttpClientFactory _factory;

    public BillingGateway(IHttpClientFactory factory) => _factory = factory;

    public async Task<Invoice> Fetch(Guid id)
    {
        var client = _factory.CreateClient("billing");
        var response = await client.GetAsync($"/invoices/{id}");
        response.EnsureSuccessStatusCode();
        return (await response.Content.ReadFromJsonAsync<Invoice>())!;
    }
}
```

Surface:

```json
{
  "id": "external-call-billing-invoice-fetch",
  "kind": "external-call-out",
  "identifier": "billing GET /invoices/{id}",
  "handler": "Gateways/BillingGateway.cs:Fetch",
  "touches": [
    "Gateways/BillingGateway.cs",
    "Models/Invoice.cs"
  ],
  "declared-at": ["Gateways/BillingGateway.cs:9"]
}
```

## `handler` resolution

`handler` answers "where does this surface's implementation live?". One value per surface, formatted `<file>:<symbol>` with the file path relative to the source root.

- **Controllers (attribute or conventional routing).** Containing class file + the action method name, e.g. `Controllers/UsersController.cs:RegisterUser`. The class file — never the controller-discovery glue (`MapControllers()`, route conventions, `Program.cs`) — is the handler.
- **Minimal API endpoints.** Containing file (typically `Program.cs`) + a synthetic suffix `<http-verb>-<route>` derived from the registration: lowercase verb, route segments joined with `-`, route parameters preserved without the braces, and `/` segments replaced with `-`. For `app.MapGet("/users", …)` use `Program.cs:get-users`; for `app.MapGet("/users/{id}", …)` use `Program.cs:get-users-id`. When the handler is a named method, prefer that method's name (`Program.cs:GetUsers`) over the synthetic suffix.
- **MassTransit consumers.** The consumer class file + `Consume`, e.g. `Consumers/UserCreatedConsumer.cs:Consume`.
- **MediatR notification handlers.** The handler class file + `Handle`, e.g. `Handlers/UserCreatedHandler.cs:Handle`.
- **MassTransit publishers (`message-pub`).** The class file + the method that owns the call site, e.g. `Services/OrderService.cs:PlaceOrder`. When two methods publish the same message, emit one `message-pub` per call site with separate `id` values (`message-pub-order-placed-create`, `message-pub-order-placed-replay`, …).
- **Hangfire / Quartz.** The job's `Run` / `Execute` method on the job class file, e.g. `Jobs/InvoiceJob.cs:Run`. The registration call site (`RecurringJob.AddOrUpdate(...)` / `ScheduleJob<T>(...)`) is the `declared-at` entry, not the handler.
- **System.CommandLine.** The bound handler method (file + method name). When the handler is an inline lambda, fall back to the containing file + a synthetic suffix `<command-path>` (e.g. `Program.cs:users-import`).
- **Typed `HttpClient` external call-outs.** The class file that owns the call site + the method that performs the call, e.g. `Gateways/BillingGateway.cs:Fetch`.

## `touches[]` resolution

`touches[]` answers "what files does the implementation reach when invoked?". Static, file-level reach analysis:

1. Start from the handler file.
2. Collect every `using` directive's resolved file inside the same project — both bare `using Foo.Bar;` and `using static Foo.Bar;`. Resolve to the file declaring the matching namespace / type.
3. Walk project references in the `.csproj` graph for the handler's project. **Stop at project boundaries** — files inside referenced projects are NOT added to `touches[]` for v1; their existence is captured by the project reference, not by file enumeration.
4. For each file added, recurse: collect its `using`-resolved files within the same project, breadth-first.
5. Record the closure as relative paths under the source root, alphabetically sorted, deduplicated.

Exclusions (always):

- `bin/`, `obj/`, NuGet caches (`packages/`, the user-level `~/.nuget/packages` is never inside the source root anyway), `.vs/`.
- Generated files: `*.g.cs`, `*.Designer.cs`, `*.AssemblyInfo.cs`, `obj/**/*.cs` (Razor / source-generator output).
- Test files: anything ending `*Tests.cs`, anything under a project named `*.Tests/` or `*.Test/`.

If reach analysis cannot resolve a `using` (e.g. extern alias, conditional `#if` excluded code, dynamic reflection by string), do not invent a file — leave the unresolved edge out of `touches[]` rather than hallucinating one. The handler file itself is always present in `touches[]`.

## Anti-patterns

The brief MUST NOT emit:

- **Dead controllers.** Controllers (or any kind) whose declaring assembly never reaches `MapControllers()` / `AddControllers()` lineage in any startup path. Heuristic: trace the controllers project from `Program.cs` (or `Startup.cs` legacy) — a controller in an unreferenced or never-discovered project is dead and is not a surface.
- **Test classes.** Anything matching `*Tests.cs`, `*Test.cs` under a `Tests/` directory, or any file under a project named `*.Tests/` / `*.Test/`. Even when a test class registers handlers (e.g. xUnit + WebApplicationFactory), it is not a surface.
- **Generated code.** `*.g.cs`, `*.Designer.cs`, `*.AssemblyInfo.cs`, source-generator outputs under `obj/`. EF Core migrations under `Migrations/` are typically generated and are NOT a surface; emit them only if the source explicitly registers them as a runtime entry point (rare).
- **MediatR `IRequestHandler<TRequest, TResponse>`.** That is in-process RPC, not an externally observable surface. Only `INotificationHandler<T>` becomes `message-sub`.
- **Skip-root paths.** No entry under `bin/`, `obj/`, `packages/`, `.vs/`, or any vendored / submoduled directory checked into the source root.
- **Absolute paths, drive letters, `..` traversal.** No leading `/`, no `C:\…`, no `\\?\…`, no `..` segments. Every path resolves under the source root.
- **Hallucinated framework attributes.** If the project does not import `Microsoft.AspNetCore.Mvc`, do not emit controller surfaces from a class that happens to have a `[ApiController]`-shaped attribute from a different namespace. Same for `MassTransit`, `MediatR`, `Hangfire`, `Quartz`, `System.CommandLine` — the brief enumerates surfaces from frameworks the project actually depends on.
- **Conventional routing assumptions when attribute routing is in effect.** Attribute routes (`[Route]`, `[HttpGet("…")]`) override conventional routes registered via `MapControllerRoute(...)`. When a controller carries any verb attribute, derive the route from the attribute templates, never from the conventional pattern. Emit conventional-route surfaces only for actions whose class and method carry no route template attributes at all.
- **Cross-project file paths in `touches[]`.** A `using` that resolves into a referenced project crosses a project boundary; do not include the foreign file. The reference itself is the recorded coupling.
