// Chạy được sau khi người dùng xác nhận trên bề mặt tin cậy (tech-stack.md §4). Nạp cùng forbid.cedar.
@id("confirm-low-medium")
permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { ["low", "medium"].contains(resource.risk) };

@id("confirm-high-local")
permit (principal is Agent, action == Action::"invoke", resource is Tool)
when { resource.risk == "high" && ["local", "voice"].contains(context.channel) };
