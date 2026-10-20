// Tự chạy, không hỏi (tech-stack.md §4). Nạp cùng forbid.cedar.
@id("auto-low")
permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "low" && resource.reversible
       && context.autonomy >= 1 && !context.tainted && !resource.always_confirm
       && !resource.system_scope };

@id("auto-medium")
permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "medium" && resource.reversible
       && context.autonomy >= 2 && !context.tainted && !resource.always_confirm
       && !resource.system_scope };
