# Phase 3 — Danh sách giả định (bản nháp, chờ bằng chứng)

Ký hiệu nguồn gốc: [T] = Thiệp nêu; [C] = Claude đề xuất trong tài liệu cũ; [N] = ngầm định, chưa ai nói ra.

| # | Giả định | Gốc | Có cần thiết? | Đảo ngược thì sao? |
|---|---|---|---|---|
| A1 | Một công ty có thể "tự vận hành" phần lớn bằng agent và vẫn sinh lời | [T] | Đây là mục tiêu, không phải giả định — nhưng mức tự vận hành khả thi là câu hỏi thực nghiệm | Nếu chỉ tự động được 50–70% thì mô hình đúng là "người sáng lập + đòn bẩy AI", không phải "công ty tự hành" |
| A2 | Nút thắt chính là chi phí thực hiện công việc | [N] | Không — với sản phẩm số, nút thắt thường là phân phối/tìm khách và chọn đúng vấn đề | Nếu nút thắt là phân phối, đầu tư vào agent sản xuất là tối ưu sai chỗ |
| A3 | Rẻ hơn = lợi thế cạnh tranh | [N] | Không chắc: khi mọi đối thủ cũng dùng AI, chi phí sản xuất thấp bị cạnh tranh hết; lợi thế chuyển sang phân phối, dữ liệu riêng, niềm tin | Nếu đúng là bị cạnh tranh hết, lợi nhuận đến từ tài sản khó sao chép, không từ năng suất |
| A4 | Tổ chức theo phòng ban agent (CEO agent + phòng ban) là kiến trúc đúng | [C] | Có dấu hiệu là thừa kế truyền thống | Thay bằng luồng công việc (workflow) + chính sách + người chịu trách nhiệm theo quyết định |
| A5 | Cần một "CEO agent" | [C] | Nghi ngờ: chiến lược/phân bổ vốn là quyết định Loại 1, cần người; agent hữu ích làm phân tích, không làm "CEO" | Bỏ CEO agent; thay bằng agent lập báo cáo danh mục + người quyết |
| A6 | Một người chủ sở hữu duyệt vài giờ/tuần là đủ | [C] | Chưa biết; có thể là nút thắt lớn nhất | Nếu duyệt là nút thắt, phải giảm số quyết định cần duyệt (thiết kế sản phẩm ít rủi ro) chứ không chỉ tăng tự chủ |
| A7 | Agent đủ tin cậy để làm việc nhiều bước mà không giám sát liên tục | [N] | Phải có bằng chứng (chờ nghiên cứu 02) | Nếu không, cần thiết kế việc thành các bước ngắn có kiểm tra tự động |
| A8 | Người làm và người kiểm dùng mô hình khác nhau là đủ độc lập | [C] | Hợp lý nhưng chưa kiểm chứng; nhiều lỗi tương quan giữa các mô hình | Có thể cần kiểm tra dựa trên dữ liệu/công cụ xác định (test, đối chiếu nguồn) thay vì LLM kiểm LLM |
| A9 | Làm nội dung trước vì rủi ro thấp, vốn ít | [C] | Chưa kiểm chứng; nội dung có thể là thị trường khó nhất năm 2026 (AI Overviews, chính sách spam, cung bùng nổ) | Phần mềm hoặc dịch vụ có thể đến doanh thu nhanh hơn |
| A10 | Micro-SaaS danh mục nhiều sản phẩm nhỏ khả thi | [C] | Chưa có dữ liệu phân phối kết quả | Có thể phân tán nguồn lực; một sản phẩm tập trung có thể tốt hơn |
| A11 | Doanh thu định kỳ là nền tảng tốt nhất | [C] | Hợp lý cho dự báo, nhưng không phải điều kiện bắt buộc | Dịch vụ theo kết quả (productized service) có thể ra tiền nhanh hơn để nuôi thử nghiệm |
| A12 | Thị trường mục tiêu chưa xác định (VN hay toàn cầu) | [N] | Quyết định này ảnh hưởng mọi thứ: giá, thanh toán, luật, ngôn ngữ | — |
| A13 | Chi phí AI là chi phí biến đổi chính, chi phí người nhỏ | [C] | Bỏ sót thời gian của chủ sở hữu và chi phí sửa lỗi | Nếu tính thời gian chủ sở hữu theo giá thị trường, có thể không có lãi kinh tế |
| A14 | Dùng chung dịch vụ cho danh mục sẽ tiết kiệm | [C] | Đúng nếu có nhiều đơn vị; ở giai đoạn 1 đơn vị thì là thiết kế sớm | Xây cho một đơn vị trước, tách dùng chung khi có đơn vị thứ hai |
| A15 | Xây "Company OS" trước rồi mới chạy kinh doanh | [C] | Nghi ngờ: xây nền tảng trước khi có khách là rủi ro sản phẩm kinh điển | Chạy kinh doanh bằng công cụ có sẵn + thủ công, chỉ tự động hoá phần đã lặp lại |
| A16 | Agent có thể làm compliance như một phòng ban | [C] | Nghĩa vụ pháp lý nên là ràng buộc trong mọi luồng + luật sư theo việc | — |
| A17 | Mức tự chủ tăng dần theo dữ liệu lỗi | [C] | Hợp lý, nhưng cần đủ khối lượng giao dịch để có số liệu thống kê | Ở khối lượng thấp, nâng tự chủ dựa trên phân loại rủi ro + khả năng hoàn tác hơn là tỷ lệ lỗi |
| A18 | Công khai nội dung AI không làm giảm giá trị thương mại | [N] | Chưa biết; bắt buộc ở nhiều nơi nên không phải lựa chọn | — |
| A19 | Thiệp có chuyên môn kỹ thuật để duyệt code/hệ thống | [N] | Chưa biết | Nếu không, reviewer kỹ thuật thuê ngoài là chi phí ngày 1 |
| A20 | Mục tiêu là lợi nhuận bền vững chứ không phải tăng trưởng nhanh/gọi vốn | [N] | Cần xác nhận; ảnh hưởng đến chiến lược | — |
