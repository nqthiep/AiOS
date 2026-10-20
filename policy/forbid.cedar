// Nạp vào cả hai tập auto và confirmable (tech-stack.md §4, design.md §18.3, integration.md §8).
@id("forbid-high-remote-schedule")
forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "high" && ["remote", "schedule"].contains(context.channel) };

@id("forbid-remote-schedule-tools")
forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { context.channel == "remote" && resource.name like "schedule.*" };

@id("forbid-trusted-core")
forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.writes_trusted_core };

@id("forbid-system-scope-non-admin")
forbid (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.system_scope && !principal.is_admin };
