# Thiết kế công ty AI-native tự hành

Bản gộp rev 11 và rev 12 · 18/10/2026 · @Thiệp Nguyễn Quang

**Trạng thái.** Tài liệu này gộp hai tab của Claude Doc [Thiết kế công ty AI-native tự hành](https://claude.ai/code/artifact/b8d698fa-15aa-4f25-9c41-6c9845c4ea6c): rev 11 (bản đang dùng) và Rev 12 (đề xuất, đã sửa ngày 16/10 theo [phản biện](../critique/phan-bien-rev12.md)). Phần lõi của rev 11 giữ nguyên. Các phần lấy từ rev 12 (chuỗi đầu tiên tới đồng tiền đầu tiên, danh mục công cụ, công thức, nguồn gốc tệp, loại cược, lời lỗ từng sản phẩm, cổng duyệt, thứ tự xây giai đoạn A và B) vẫn chờ anh duyệt câu 1 ở mục "Anh cần quyết". Lịch sử các lần sửa nằm ở cuối tài liệu.

## Tóm tắt

Tài liệu này thiết kế **chính công ty**, không chọn sản phẩm cho công ty. Chọn ngách, chọn sản phẩm, định giá, bỏ sản phẩm là việc công ty tự làm khi chạy. Mục tiêu là một mô hình tổ chức và mô hình hoạt động đủ cụ thể để lập trình thành source code, cho hai loại công ty: sản xuất phần mềm và sản xuất nội dung số.

CompanyOS thôi gắn cứng vào việc viết bài. Mỗi công cụ (mô hình AI, ứng dụng dòng lệnh, API, MCP) là một mục trong danh mục, ghi rõ nó làm được gì, gọi bằng cách nào, tốn bao nhiêu và có được bán thương mại sản phẩm làm ra không. Mỗi sản phẩm là một công thức ghép các công cụ đó. Mọi tệp làm ra mang theo nguồn gốc, nên code tự gắn nhãn AI, tự chặn vi phạm đã biết trước, và anh chỉ phải duyệt việc lần đầu làm.

**Cách làm.** Đi từ nguyên lý, không từ sơ đồ tổ chức có sẵn. Trước hết xác định những việc mọi doanh nghiệp phải làm để tồn tại và sinh lời. Sau đó hỏi mỗi việc nên do code, agent, dịch vụ ngoài hay con người làm, dựa trên bằng chứng về năng lực agent hiện nay. Phần nghiên cứu và bằng chứng dùng lại từ [báo cáo trước, nay đã được thay bằng tài liệu này](https://claude.ai/code/artifact/7aef223a-498e-4aa7-adb1-e54b0328ebaa); mã E1–E12 giải thích ở [báo cáo giữa kỳ](https://claude.ai/code/artifact/c56ff2a8-3090-4cee-8d59-d30781b23aa1).

**Kết luận chính.**

1. Công ty tự hành nên được xây như một **hệ thống tự điều chỉnh**: năm lớp chức năng (vận hành, phối hợp, điều khiển, thích nghi, chính sách) theo Mô hình Hệ thống Sống còn (Viable System Model) của Stafford Beer. Đây là lý thuyết về điều kiện để một tổ chức tự duy trì được, không phải một sơ đồ phòng ban.
2. **Agent làm việc, code giữ luật, người giữ chính sách.** Tiền, quyền, sổ sách, ngân sách và điều phối nằm trong code xác định. Agent làm các việc cần hiểu ngôn ngữ và kiểm tra được. Không có CEO agent, vì bằng chứng cho thấy agent yếu nhất ở việc tự điều hành kinh doanh dài hạn (E2, E3).
3. **Công ty tự tìm ngách bằng tiền thật.** Cơ chế trung tâm là danh mục cược: mỗi ý tưởng có ngân sách, bán thử trước khi làm, dừng theo quy tắc. Ngân sách tự chảy về đơn vị có biên đóng góp cao.
4. **Tối ưu chi phí bằng cơ chế, không bằng cắt người.** Định tuyến mô hình rẻ trước, dừng cược nhanh, mua thay vì xây những thứ không tạo khác biệt, và mỗi vai trò agent phải chứng minh giá trị lớn hơn chi phí token cộng chi phí kiểm tra.
5. **Phần con người không bỏ được, nhưng nhỏ.** Một người chịu trách nhiệm pháp lý, đặt chính sách, và duyệt số ít quyết định không đảo ngược được. Luật Việt Nam yêu cầu người đại diện là cá nhân \[Đ\]. Thiết kế nhắm để phần việc này giảm dần khi có số liệu chứng minh an toàn.
6. **Mọi loại sản phẩm dùng chung một lõi.** Phần mềm, nội dung chữ, nhạc, video khác nhau ở dây chuyền sản xuất và cách kiểm tra chất lượng, giống nhau ở mọi thứ còn lại. Lõi lập trình một lần; mỗi loại sản phẩm là một bộ công thức và một hiến chương mẫu.
7. **Công cụ là dữ liệu, không phải code.** Mỗi công cụ là một mục trong danh mục, ghi giấy phép và quyền bán đầu ra; mỗi tệp mang hồ sơ nguồn gốc; quyền của sản phẩm bằng quyền hẹp nhất trong các tệp góp vào. Nhờ vậy công ty tự mở rộng sang công cụ mới mà code vẫn tự chặn được vi phạm đã biết.

Nhãn: **\[Đ\]** đã kiểm chứng qua nguồn, **\[S\]** suy luận, **\[GT\]** giả thuyết cần thử, **\[LT\]** lý thuyết tổ chức kinh điển trích theo hiểu biết chung, chưa đọc lại bản gốc trong đợt này.

## Nguyên lý vận hành của doanh nghiệp

Câu hỏi ở phần này: bỏ hết chức danh và phòng ban, một doanh nghiệp phải làm những việc gì để tồn tại và có lãi? Danh sách dưới đây rút từ các lý thuyết tổ chức và kinh tế học doanh nghiệp được dùng rộng rãi. Mỗi nguyên lý đều đúng với công ty có người lẫn công ty toàn agent; thứ thay đổi là cách thực hiện.

| # | Nguyên lý | Nội dung | Cơ sở | Nếu công ty không làm |
| --- | --- | --- | --- | --- |
| N1 | Tạo giá trị | Tìm vấn đề mà người khác sẵn sàng trả tiền để giải quyết | Mô hình kinh doanh: tạo, giao, thu giá trị \[LT\] | Làm ra thứ không ai mua |
| N2 | Giao giá trị | Sản xuất và giao đúng chất lượng, đúng hạn, lặp lại được | Chuỗi giá trị Porter \[LT\] | Mất khách, hoàn tiền, mất uy tín |
| N3 | Thu giá trị | Định giá, thu tiền, giữ khách; doanh thu phải vượt chi phí biến đổi | Kinh tế đơn vị \[LT\] | Bán nhiều vẫn lỗ |
| N4 | Sống sót về tiền | Không bao giờ hết tiền trước khi tìm ra cách có lãi | Quản trị dòng tiền \[LT\] | Công ty chết dù ý tưởng đúng |
| N5 | Phân bổ nguồn lực | Đưa tiền và công sức vào nơi lợi nhuận biên cao nhất, rút khỏi nơi thấp | Phân bổ vốn; danh mục dự án \[LT\] | Nuôi mãi sản phẩm thua lỗ |
| N6 | Nhắm vào nút thắt | Thông lượng cả hệ thống do một điểm nghẽn quyết định; tối ưu chỗ khác là lãng phí | Lý thuyết ràng buộc, Goldratt \[LT\] | Sản xuất nhanh hơn nhưng không bán thêm được |
| N7 | Phối hợp | Nhiều người làm phải khớp nhau; tổ chức lớn dùng quy trình chuẩn và đầu ra chuẩn thay cho giám sát trực tiếp | Cấu hình Mintzberg \[Đ qua nghiên cứu 03\] | Xung đột, làm trùng, vòng lặp |
| N8 | Kiểm soát | Tách quyền: người làm, người duyệt, người ghi sổ, người đối soát là khác nhau; có giới hạn và kiểm toán | COSO, phân tách nhiệm vụ \[Đ qua nghiên cứu 03\] | Gian lận, sai sót không ai phát hiện |
| N9 | Thích nghi | Cảm nhận môi trường (thị trường, đối thủ, luật, công nghệ) và đổi hướng trước khi quá muộn; học từ sai lầm, kể cả sửa luật chơi | VSM hệ 4; học hai vòng, Argyris \[LT\] | Làm tốt việc đã lỗi thời |
| N10 | Bản sắc và chính sách | Mục tiêu, giá trị, điều không bao giờ làm; cân bằng giữa vận hành hôm nay và thích nghi ngày mai | VSM hệ 5 \[LT\] | Mỗi bộ phận tối ưu theo một hướng |
| N11 | Trách nhiệm và lòng tin | Có người chịu trách nhiệm trước pháp luật và khách; giữ lời hứa | Luật Doanh nghiệp Đ.12 \[Đ\]; Moffatt v. Air Canada \[Đ\] | Không ký được hợp đồng, không ai tin |
| N12 | Ranh giới công ty | Việc nào tự làm, việc nào mua ngoài: tự làm khi chi phí phối hợp nội bộ thấp hơn chi phí giao dịch trên thị trường | Coase, Williamson \[Đ qua nghiên cứu 03\] | Xây thứ nên mua, hoặc phụ thuộc thứ nên tự giữ |

### Vì sao chọn Mô hình Hệ thống Sống còn làm khung

Các lý thuyết trên mô tả từng mặt. Công ty tự hành cần một khung nói rõ các mặt ghép với nhau thế nào để hệ thống tự duy trì mà không cần người điều khiển từng bước. Mô hình Hệ thống Sống còn (VSM) của Stafford Beer được xây đúng cho câu hỏi này \[LT\]. Nó nói rằng mọi tổ chức tự duy trì được đều có năm hệ chức năng:

- **Hệ 1, vận hành:** các đơn vị trực tiếp tạo và giao giá trị (N1–N3).
- **Hệ 2, phối hợp:** giữ các đơn vị không dẫm chân nhau (N7).
- **Hệ 3, điều khiển:** phân bổ nguồn lực, đặt chỉ tiêu, kiểm soát; kèm **hệ 3\*** kiểm toán độc lập bằng lấy mẫu (N4, N5, N6, N8).
- **Hệ 4, thích nghi:** nhìn ra ngoài và về tương lai (N9).
- **Hệ 5, chính sách:** bản sắc và luật chơi cao nhất (N10, N11).

VSM còn có một ý quan trọng cho thiết kế này: **định luật đa dạng cần thiết của Ashby** \[LT\]. Bộ phận điều khiển phải có đủ độ đa dạng phản ứng ngang với độ đa dạng tình huống nó gặp. Với một người chủ duy nhất, cách duy nhất là giảm độ đa dạng đi lên tới người: tự động xử lý tình huống thường, chỉ đưa lên ngoại lệ đã lọc. Đây là lý do lý thuyết cho việc quản lý theo ngoại lệ và bảng quyền A–D.

VSM không phải là sơ đồ cấp trên cấp dưới. Năm hệ là năm chức năng, mỗi chức năng có thể do code, agent hoặc người đảm nhận. Trước đây VSM khó áp dụng vì tốn kém đo đạc và truyền tin; trong công ty chạy bằng phần mềm, mọi luồng thông tin đã là dữ liệu nên chi phí này gần như bằng 0 \[S\].

**Điều bằng chứng về agent thay đổi trong các nguyên lý này** \[S, dựa trên E1–E12\]:

- N7: phối hợp bằng “thương lượng lẫn nhau” là nơi agent hỏng nhiều nhất (vòng lặp, trôi mục tiêu). Phối hợp phải bằng quy trình và đầu ra chuẩn hóa trong code.
- N8: cùng một mô hình vừa làm vừa kiểm là không tách quyền. Tách quyền phải tách cả mô hình, dữ liệu và khóa truy cập.
- N6: khi sản xuất gần như miễn phí, nút thắt chuyển sang phân phối và kiểm tra chất lượng.
- N12: chi phí phối hợp nội bộ giảm mạnh, nên công ty rất nhỏ vẫn làm được nhiều việc. Nhưng chi phí kiểm tra và trách nhiệm pháp lý không giảm, nên những thứ đó vẫn phải nằm trong công ty.

## Từ nguyên lý đến cơ chế tự hành

Mỗi nguyên lý được biến thành một cơ chế chạy được: có người hoặc thứ thực hiện, có tín hiệu đo, và có quy tắc tự điều chỉnh khi tín hiệu lệch. Một cơ chế thiếu một trong ba phần đó thì chưa tự hành được.

| Nguyên lý | Cơ chế trong công ty AI-native | Thực hiện bởi | Tín hiệu đo | Tự điều chỉnh khi lệch |
| --- | --- | --- | --- | --- |
| N1 Tạo giá trị | **Động cơ tìm cơ hội:** quét tín hiệu nhu cầu công khai, viết thẻ cơ hội, chấm điểm theo thang cố định | Agent tìm kiếm (chỉ đọc) + thang chấm bằng code | Tỷ lệ thẻ qua cổng 2 (có người trả tiền) | Hiệu chỉnh trọng số thang chấm theo cược nào thật sự ra tiền |
| N1 + N3 | **Bán trước khi làm:** mỗi cơ hội phải có người lạ trả tiền trước khi được cấp ngân sách sản xuất. Các loại cược khác (xây khán giả, doanh thu nền tảng, bán quyền sử dụng, nhận đặt làm) có cổng riêng, xem mục Kênh, cách thu tiền và loại cược | Agent làm trang bán; cổng bằng code | Số thanh toán thật trong 14 ngày | Dưới ngưỡng thì dừng và hoàn tiền tự động |
| N2 Giao giá trị | **Dây chuyền sản xuất có bài nghiệm thu:** việc chia nhỏ, mỗi việc có tiêu chí đạt, người làm khác người kiểm | Agent làm + bộ kiểm tra xác định + agent kiểm khác hãng | Tỷ lệ đạt lần đầu; lỗi lọt tới khách; hoàn tiền | Lỗi lọt tăng thì tăng tỷ lệ lấy mẫu và thêm bài eval; lần làm thứ 3 vẫn hỏng thì leo thang |
| N3 Thu giá trị | **Thu tiền qua bên bán hộ; thử giá trong khung đã duyệt** | Dịch vụ ngoài (Paddle, Polar, VietQR) + agent thử giá | Biên đóng góp mỗi đơn vị; tỷ lệ rời bỏ | Biên âm hai tháng thì đơn vị vào diện xem xét dừng |
| N4 Sống sót | **Ngân sách cứng và đường băng tiền:** trần token theo tháng, theo đơn vị, theo việc; trần cứng ở nhà cung cấp | Code + cấu hình nhà cung cấp | Số tháng còn sống = quỹ / mức đốt ròng | Dưới ngưỡng thì chế độ tiết kiệm: ngừng cược mới, chỉ giữ đơn vị có lãi |
| N5 Phân bổ | **Bộ phân bổ theo quy tắc:** ngân sách kỳ sau = sàn + hệ số × biên đóng góp kỳ trước của từng đơn vị; phần thăm dò cố định cho cược mới | Code; agent chỉ viết báo cáo giải thích | Biên đóng góp trên mỗi USD token, theo đơn vị | Tự động mỗi kỳ; thay đổi quy tắc là nhóm C |
| N6 Nút thắt | **Đo thông lượng từng bước của dây chuyền và của phễu bán**, dồn ngân sách vào bước nghẽn | Code đo; agent phân tích | Thời gian chờ, tỷ lệ chuyển đổi từng bước | Bước nghẽn được ưu tiên trong hàng đợi; không tăng sản xuất khi phân phối nghẽn |
| N7 Phối hợp | **Bộ điều phối xác định:** hàng đợi việc, luồng viết sẵn, khóa tài nguyên, đầu ra có lược đồ; agent không nhắn tin tự do với nhau | Code (workflow engine) | Việc kẹt, vòng lặp, xung đột tài nguyên | Ngắt vòng lặp theo số lượt; việc kẹt quá hạn thì hủy hoặc leo thang |
| N8 Kiểm soát | **Tách quyền trong code:** mỗi vai trò một khóa, quyền tối thiểu; sổ cái chỉ ghi từ dữ liệu nguồn; bảng quyền A–D; **kiểm toán độc lập** bằng lấy mẫu ngẫu nhiên và lỗi mồi | Code + agent kiểm toán khác hãng + anh xem mẫu | Tỷ lệ bắt lỗi mồi; chênh lệch đối soát | Bắt lỗi mồi dưới ngưỡng thì hạ quyền loại việc đó về nhóm C |
| N9 Thích nghi | **Quét môi trường và học từ kết quả:** theo dõi đối thủ, giá mô hình, chính sách nền tảng, luật mới; mỗi cược dừng để lại bài học; mỗi lỗi thành bài eval | Agent quét + bộ nhớ tổ chức | Số thay đổi môi trường ảnh hưởng đơn vị đang chạy | Đề xuất sửa chính sách gửi anh (học hai vòng); sửa quy trình trong khung thì tự làm qua eval |
| N10 Chính sách | **Hiến chương công ty:** tệp cấu hình có phiên bản ghi mục tiêu, giới hạn chi, danh mục cấm, bảng quyền, ngưỡng | Anh soạn và duyệt; code thực thi | Số lần hành động bị chặn bởi hiến chương | Chỉ anh sửa; agent được đề xuất sửa kèm số liệu |
| N11 Trách nhiệm | **Người chịu trách nhiệm có tên, lời hứa chỉ từ chính sách đã duyệt, nhật ký không sửa được** | Anh; code ghi nhật ký | Khiếu nại, tranh chấp | Khiếu nại pháp lý luôn lên anh; giữ lời hứa đã đưa ra rồi sửa nguồn lỗi |
| N12 Ranh giới | **Quy tắc tự làm hay mua:** mua thứ có trách nhiệm pháp lý hoặc không tạo khác biệt (thanh toán, thuế, hosting); giữ trong nhà dữ liệu, bộ eval, prompt, quy trình | Quy tắc trong hiến chương | Chi phí dịch vụ ngoài so với chi phí tự làm | Nhà cung cấp mới là nhóm C |

**Ba cơ chế này là xương sống** \[S\]: bán trước khi làm (N1, N3), bộ phân bổ theo quy tắc (N5), và kiểm toán độc lập bằng lỗi mồi (N8). Thiếu cơ chế thứ nhất, công ty tự hành sẽ sản xuất rất hiệu quả những thứ không ai mua. Thiếu cơ chế thứ hai, nó không tự rút khỏi chỗ lỗ. Thiếu cơ chế thứ ba, không ai biết chất lượng thật đang ở đâu khi anh không nhìn.

## Giới hạn thực tế của công cụ và kênh

Sức sáng tạo không phải giới hạn duy nhất. Sáu giới hạn dưới đây chặn trước, và thiết kế này xây quanh chúng. Nhãn: **Đ** = đã mở trang gốc; **T** = nguồn thứ cấp (blog, trang tổng hợp); **S** = em suy luận.

| Giới hạn | Điều em kiểm được | Nhãn | Hệ quả cho thiết kế |
| --- | --- | --- | --- |
| Tiền mặt | Trước doanh thu anh có khoảng 50 USD tiền mặt. Thuê máy có card 24 GB ở RunPod: RTX 4090 giá 0,89 USD/giờ, RTX A5000 giá 0,27 USD/giờ. Wan 2.2 làm 5 giây video 720P mất dưới 9 phút trên một card phổ thông, nên 1 USD thuê RTX 4090 được khoảng 35 giây video. | Đ: giá RunPod, tốc độ Wan. S: con số 35 giây | Ưu tiên công cụ miễn phí hoặc trả theo lần dùng. Gói tháng là việc anh duyệt, và chỉ mở khi cược đã có tín hiệu người mua. Với trần 1 USD mỗi ngày, video AI thuê máy chỉ đủ làm thử, không đủ làm nhiều. |
| Cách gọi công cụ | Agent chỉ dùng được công cụ có dòng lệnh, API hoặc MCP. Các app Craft có "headless CLI for scripts and agents", nhưng cả 13 trang chỉ liệt kê CLI cho macOS. Mã nguồn PhotoCraft mở (MIT hoặc Apache-2.0) và có lệnh `photocraft-cli mcp`; em chưa thử biên dịch CLI cho Windows hay Linux. Suno không có API công khai. Điều khoản Suno cấm "data mining, robots, scraping, or similar data gathering or extraction methods"; câu này nói về thu thập dữ liệu, không nói thẳng tới agent tạo nhạc. | Đ | Mỗi công cụ ghi rõ cách gọi. Công cụ chỉ có giao diện bấm tay không vào dây chuyền tự động. Anh chọn dùng Suno qua agent điều khiển trình duyệt; em đọc điều khoản không cấm thẳng việc này \[S\]. Rủi ro còn lại: Suno được khóa tài khoản nếu cho là trái "letter or spirit" của điều khoản, và được khóa tài khoản, xóa mọi tệp "in its sole discretion"; quyền bán chỉ có khi tải qua nút tải chính thức. Vì vậy công ty tải bài về ngay sau khi tạo (xem Danh mục). |
| Quyền bán sản phẩm làm ra | Suno gói miễn phí chỉ cho dùng "personal and non-commercial"; gói trả phí mới được bán, và Suno "makes no representation or warranty to you that any copyright will vest in any Output". XTTS-v2 (giọng nói, có trên GitHub) dùng giấy phép không thương mại. ACE-Step (nhạc) dùng MIT; Wan 2.2 (video) dùng Apache 2.0. Kokoro (giọng nói) dùng Apache 2.0 nhưng chỉ có 9 ngôn ngữ, không có tiếng Việt. | Đ: Suno, ACE-Step, Wan 2.2, danh sách giọng Kokoro. T: XTTS-v2, giấy phép Kokoro | Có trên GitHub không có nghĩa là được bán đầu ra. Danh mục ghi giấy phép của công cụ và của đầu ra; code chặn bán đầu ra từ công cụ không thương mại. Hiện chưa có công cụ ảnh AI và giọng tiếng Việt được bán, nên công thức cần hai năng lực này chưa chạy được. |
| Máy chạy | Mô hình mở cần card đồ họa. README của Wan 2.2: bản TI2V-5B cần ít nhất 24 GB VRAM (ví dụ RTX 4090), bản A14B cần ít nhất 80 GB khi chạy trên một card; có tùy chọn giảm bộ nhớ, em chưa thử. ACE-Step cần dưới 4 GB. Em chưa biết máy anh có card gì. | Đ | Mỗi công cụ ghi nơi chạy. Video AI chỉ vào dây chuyền khi máy anh có card từ 24 GB, hoặc anh duyệt thuê máy theo giờ (tốn tiền mặt). |
| Tự kiểm chất lượng | Code hiện chỉ kiểm được chữ: đối chiếu nguồn, người kiểm khác hãng, bộ lỗi mồi cho chữ. Bộ lỗi mồi chữ chưa có ca sạch, nên chưa đo được tỉ lệ báo nhầm. Ảnh, tiếng, video chưa có bộ kiểm nào. | Đ (code) | Loại media nào chưa có bộ kiểm đạt chuẩn (bắt được ít nhất 80% lỗi mồi, báo nhầm không quá 20%) thì mỗi lần phát hành vẫn chờ anh xem. |
| Người mua và kênh | YouTube không trả tiền cho nội dung "inauthentic", gồm nội dung làm hàng loạt theo khuôn. Mỗi kênh còn một cửa công ty không tự mở được (xem phần Kênh). Ai cũng có cùng công cụ, nên làm ra thì rẻ dần, còn bán được thì không rẻ đi. | Đ: chính sách YouTube, cửa của từng kênh. S: câu cuối | Tiền nằm ở chỗ tìm ra người cần và kênh tới họ, không ở số công cụ. Thiết kế giữ cổng thử nhu cầu trước khi làm nhiều, và đo lời lỗ từng sản phẩm. |

**Về việc anh là cổng pháp lý.** Em đồng ý anh duyệt cuối. Em vẫn giữ hai thứ trong code, không phải vì nghi ngờ anh:

1. Thời gian của anh có hạn. Nếu mọi sản phẩm đều tới tay anh, anh thành nút thắt và công ty không tự chạy được. Nên code tự chặn những vi phạm đã biết trước, và chỉ đưa anh việc lần đầu làm.
2. Có vi phạm không nhìn thấy trong sản phẩm. Một đoạn giọng đọc làm bằng XTTS-v2 nghe vẫn bình thường, nhưng giấy phép của nó không cho bán. Anh chỉ bắt được nếu hồ sơ ghi rõ công cụ nào làm ra nó, nên mọi tệp phải mang nguồn gốc.

## Mô hình tổ chức

Tổ chức được chia theo chức năng sống còn, không theo phòng ban. Mỗi hệ ghi rõ ai đảm nhận: người, agent, code hay dịch vụ ngoài.

Lệnh và luật đi từ trên xuống. Thông tin đi lên chỉ dưới dạng ngoại lệ đã lọc: hệ 3 gom chỉ số của mọi đơn vị và chỉ đưa lên anh những gì vượt ngưỡng. Hệ 3\* đi thẳng xuống hệ 1 để lấy mẫu, không qua báo cáo của chính đơn vị, vì đơn vị tự báo cáo về mình là không độc lập (E5).

### Vai trò agent

Mỗi vai trò là một bản mô tả có phiên bản: đầu vào, đầu ra theo lược đồ, công cụ được dùng, quyền, ngân sách mỗi việc, bài nghiệm thu, mô hình mặc định, và khi nào leo thang. Một vai trò chỉ được tạo khi chứng minh được giá trị lớn hơn chi phí token cộng chi phí kiểm tra của nó.

| Vai trò | Hệ | Làm gì | Không được làm | Ai kiểm |
| --- | --- | --- | --- | --- |
| Trinh sát thị trường | 4 | Đọc dữ liệu công khai, viết thẻ cơ hội | Đăng bài, chi tiền, xem dữ liệu khách | Thang chấm bằng code |
| Trinh sát công cụ | 4 | Tìm công cụ mới, mở trang giá, giấy phép, điều khoản; điền sẵn mục danh mục ở trạng thái "đề xuất" kèm câu trích nguyên văn | Tự duyệt công cụ; bán đầu ra khi chưa có quyền | Anh duyệt từ câu trích (nhóm C) |
| Trinh sát môi trường | 4 | Theo dõi luật, chính sách nền tảng, giá mô hình, đối thủ | Thay đổi cấu hình | Anh đọc tóm tắt khi có thay đổi lớn |
| Người bán thử | 1 | Làm trang bán, ưu đãi, bài giới thiệu cho cược | Hứa ngoài chính sách; đánh giá giả | Cổng kiểm tra nội dung + luật đạo đức |
| Người làm (phần mềm hoặc nội dung) | 1 | Thực hiện việc nhỏ có bài nghiệm thu | Giữ khóa production; tự duyệt việc của mình | Người kiểm |
| Người kiểm | 1 | Chạy test, đối chiếu nguồn, chấm theo tiêu chí; dùng mô hình khác hãng với người làm | Sửa việc mình kiểm | Kiểm toán (lỗi mồi) |
| Người phát hành | 1 | Triển khai hoặc đăng bài có tự hoàn tác | Xóa dữ liệu; migration không qua hai khóa | Giám sát sau phát hành |
| Người phân phối | 1 | Soạn và đăng bài trên kênh đã duyệt | Mở kênh mới; giả làm người thật | Cổng kiểm tra + giới hạn tốc độ |
| Hỗ trợ khách | 1 | Trả lời từ chính sách đã duyệt, tự giới thiệu là AI, hoàn tiền dưới trần | Hứa ngoài chính sách | Lấy mẫu hội thoại |
| Kế toán viên (chỉ đọc) | 3 | Đối soát sổ cái với dữ liệu bên bán hộ, ngân hàng | Chuyển tiền; sửa sổ | Đối soát hai chiều bằng code |
| Phân tích danh mục | 3 | Viết giải thích cho quyết định của bộ phân bổ; đề xuất | Tự đổi ngân sách | Anh khi đề xuất thuộc nhóm C |
| Kiểm toán viên | 3\* | Lấy mẫu ngẫu nhiên, cài lỗi mồi, đo tỷ lệ bắt lỗi | Sửa việc được kiểm | Anh xem báo cáo kiểm toán |
| Người soạn hồ sơ | 5 | Chuẩn bị hồ sơ cho việc nhóm C: tóm tắt, bằng chứng, lựa chọn | Quyết định | Anh |

Không có vai trò CEO, quản lý hay trưởng phòng. Những gì một quản lý làm được tách ra: phân bổ (code), điều phối (code), kiểm soát chất lượng (người kiểm và kiểm toán), quyết định khó đảo ngược (anh).

### Con người

- **Chủ sở hữu (anh):** hệ 5. Viết và sửa hiến chương; duyệt nhóm C; ký mọi thứ thuộc nhóm D; xử lý sự cố mức 3. Không đọc nhật ký hằng ngày, chỉ đọc bảng ngoại lệ và báo cáo kiểm toán.
- **Chuyên gia thuê theo việc:** kế toán, luật sư. Công ty tự soạn hồ sơ, người chuyên môn kiểm và ký.
- **Người duyệt mẫu (khi có điều kiện):** thuê khi tỷ lệ bắt lỗi mồi dưới ngưỡng, hoặc khi giờ của anh vượt ngưỡng. Trả từ doanh thu.

### Ranh giới quyền

Bảng quyền A–D dùng lại từ báo cáo trước: A tự làm; B tự làm, có kiểm mẫu; C anh duyệt; D không bao giờ giao agent. Một loại việc chuyển từ C xuống B khi có ba điều kiện: cách hoàn tác đã thử, trần thiệt hại trong code, và bài eval cho loại việc đó đã bắt được lỗi mồi. Đây là cơ chế để phần việc của con người giảm dần theo số liệu, không theo cảm giác.

### Cổng duyệt của anh

Anh nhận việc lần đầu; code tự chạy việc lặp lại đã duyệt và tự chặn vi phạm đã biết. Mỗi việc tới anh là một hồ sơ vừa một màn hình.

| Việc | Ai xử lý |
| --- | --- |
| Chạy công thức đã duyệt trong ngân sách; gọi công cụ đã duyệt; dùng lại tài sản trong kho; nghiên cứu; làm nháp; sinh worker trong trần | Công ty tự làm (nhóm B) |
| Công cụ mới; công thức mới hoặc thêm bước; kênh mới; cách thu tiền mới; ngành hoặc thị trường mới; gói tháng; thuê người | Anh duyệt (nhóm C) |
| Phát hành công khai: đăng bài, bật kiếm tiền, nhận tiền thật | Anh duyệt (nhóm C), cho tới khi anh đổi |
| Bán đầu ra của công cụ không thương mại; gọi tự động công cụ cấm bot; dùng giọng hoặc mặt người thật khi không có giấy đồng ý; đầu vào có bản quyền mà không có giấy phép; tương tác giả, đánh giá giả; nhiều tài khoản để lách giới hạn; tải trang mà robots.txt cấm; ngành cấm trong hiến chương | Code chặn, không tới anh |
| Ký hợp đồng, nộp thuế, chuyển tiền ra, mở tài khoản | Chỉ anh hoặc kế toán (nhóm D, agent không bao giờ làm) |

Danh sách chặn nằm trong hiến chương, anh sửa được. Code chỉ chặn được điều có trong hồ sơ (công cụ, giấy phép, nguồn đầu vào). Điều chỉ thấy bằng mắt, ví dụ hình giống một nhân vật có bản quyền, thì người kiểm khác hãng gắn cờ và đưa tới anh.

Hồ sơ anh nhận gồm:

1. Việc gì, và "lần đầu" ở điểm nào (công cụ, công thức, kênh, cách thu tiền).
2. Tiền dự kiến và cược liên quan.
3. Chi phí dự kiến, và chi phí thật của lần chạy nháp.
4. Công cụ dùng, giấy phép, quyền bán đầu ra, điều khoản gọi tự động, mỗi dòng một link.
5. Nguồn đầu vào và giấy phép của chúng.
6. Nhãn AI gắn ở đâu.
7. Quy định nền tảng liên quan, có link.
8. Tệp mẫu từ lần chạy nháp.
9. Code đã kiểm gì và kết quả.

Code hiện nay cho mỗi lần duyệt dùng một lần, cho đúng loại việc, người làm và lĩnh vực. Rev 12 gắn thêm mã băm của hồ sơ: hồ sơ bị sửa sau khi duyệt thì phải duyệt lại. Hàng duyệt tối đa 5 việc (hiến chương hiện tại). Hàng đầy thì công ty ngừng đề xuất việc mới và tiếp tục việc đã duyệt, để anh không bị ngợp.

Gọi đúng tên: giai đoạn đầu là dây chuyền có anh duyệt, chưa phải công ty tự hành ở khâu phát hành. Đó là chủ ý, vì loại media chưa qua bộ lỗi mồi thì chưa nên phát hành tự động. Để nhìn thấy nút thắt này, mỗi lần duyệt ghi số phút anh bỏ ra vào sổ cái, và anh đặt một mục tiêu giảm số phút đó kèm ngày (câu hỏi ở phần "Anh cần quyết").

## Mô hình hoạt động

Công ty chạy bằng năm vòng lặp lồng nhau, từ nhanh đến chậm. Vòng nhanh do agent và code tự xử lý; càng lên vòng chậm, quyết định càng ít và càng quan trọng. Vòng chậm đặt luật cho vòng nhanh; vòng nhanh gửi số liệu lên vòng chậm. Không có họp định kỳ; nhịp do sự kiện và bộ hẹn giờ quyết định.

Vòng chạy theo chiều kim đồng hồ; mỗi vòng là một kỳ phân bổ. Anh chỉ dừng vòng ở ba chỗ, và chỉ khi việc đó làm lần đầu: công cụ mới, cược có thành phần mới, phát hành công khai.

| Vòng | Nhịp | Việc | Ai quyết | Đầu ra |
| --- | --- | --- | --- | --- |
| V1 Việc | Phút đến giờ | Nhận việc → làm → kiểm → đạt thì chấp nhận; không đạt thì làm lại tối đa 2 lần rồi leo thang | Code (bài nghiệm thu) | Sản phẩm đã kiểm; chi phí ghi sổ |
| V2 Đơn vị | Theo sự kiện, hằng ngày | Mỗi đơn vị xử lý đơn hàng, ticket, bản phát hành, bài đăng; tự xếp việc theo ưu tiên của bước nghẽn | Code + agent trong quyền A, B | Số liệu đơn vị: doanh thu, chi phí, lỗi, khiếu nại |
| V3 Danh mục | Hằng tuần | Bộ phân bổ tính lại ngân sách; cược nào đến cổng thì qua hoặc dừng; mở cược mới từ thẻ cơ hội điểm cao nhất | Code theo quy tắc; vượt mặc định là nhóm C | Bảng ngoại lệ tuần cho anh |
| V4 Thích nghi | Hằng tháng | Hiệu chỉnh thang chấm ngách theo kết quả thật; cập nhật bộ eval; xem lại mô hình nào rẻ hơn mà vẫn đạt eval; báo cáo kiểm toán | Agent đề xuất; đổi quy trình trong khung thì tự làm qua eval | Đề xuất sửa chính sách kèm số liệu |
| V5 Chính sách | Khi cần, tối thiểu hằng quý | Sửa hiến chương: mục tiêu, trần chi, danh mục cấm, chuyển việc giữa nhóm C và B | Anh | Hiến chương phiên bản mới |

### Sự kiện kích hoạt

Công việc sinh ra từ sự kiện, không từ kế hoạch. Các sự kiện chính: có thanh toán mới; có yêu cầu hoàn tiền; có ticket hỗ trợ; test thất bại; bản phát hành lỗi; một cược đến cổng; chi vượt ngưỡng; trinh sát phát hiện thay đổi môi trường; kiểm toán phát hiện lỗi lọt; anh sửa hiến chương. Danh sách sự kiện đầy đủ nằm ở phần Đặc tả để lập trình.

### Leo thang

1. **Mức 1, tự xử lý:** bài nghiệm thu chặn việc hỏng và trả lại người làm.
2. **Mức 2, vào bảng ngoại lệ:** vượt ngưỡng chi phí, tỷ lệ lỗi, khiếu nại, hoặc việc hỏng sau 3 lần. Anh xem theo tuần.
3. **Mức 3, dừng và báo ngay:** dấu hiệu sự cố tiền, dữ liệu hoặc pháp lý. Code thu hồi khóa của luồng liên quan trước, rồi mới báo.

### Cơ chế tự điều chỉnh

Mỗi cơ chế dưới đây giữ một đại lượng trong khoảng an toàn mà không cần người can thiệp. Đây là phần biến công ty thành “tự hành” thay vì chỉ “tự động”. Mọi ngưỡng là giá trị khởi điểm \[GT\], nằm trong hiến chương.

| Đại lượng | Khoảng an toàn | Khi vượt |
| --- | --- | --- |
| Chi token trong ngày | ≤ 1/20 ngân sách tháng | Khóa luồng chi nhiều nhất; chờ đến ngày sau |
| Số tháng còn sống | ≥ 3 | Chế độ tiết kiệm: ngừng cược mới, đổi sang mô hình rẻ hơn đã qua eval |
| Tỷ lệ bắt lỗi mồi của người kiểm | ≥ 80% | Loại việc đó về nhóm C cho đến khi sửa xong người kiểm |
| Tỷ lệ hoàn tiền một đơn vị | ≤ 10% | Dừng bán mới ở đơn vị đó; mở việc tìm nguyên nhân |
| Biên đóng góp một đơn vị | Dương | Âm hai kỳ liền: xoay hướng một lần, rồi dừng |
| Số việc nhóm C chờ anh | ≤ 5 | Giảm số cược song song cho đến khi hàng đợi vơi |
| Thời gian chờ ở một bước dây chuyền | ≤ 2 lần trung vị | Ưu tiên bước đó trong hàng đợi; không nhận việc mới vào đầu dây chuyền |
| Số lượt gọi mô hình mỗi việc | ≤ trần của vai trò | Ngắt vòng lặp; việc vào mức 2 |

### Công ty tự cải tiến thế nào

Agent được đề xuất sửa cách công ty làm việc: prompt, quy trình, chọn mô hình, ngưỡng. Mọi thay đổi phải chạy qua bộ eval trước, so với phiên bản cũ, rồi áp dụng cho một phần nhỏ việc trước khi áp dụng toàn bộ. Thay đổi nằm trong khung của hiến chương thì tự động; thay đổi hiến chương thì lên anh. Agent không bao giờ được sửa bộ eval dùng để chấm chính nó, vì đó là cách nhanh nhất để “đạt” mà không tốt lên \[S\].

## Danh mục công cụ

Mỗi công cụ là một mục trong danh mục. Agent chỉ gọi được công cụ đã duyệt, qua bộ chuyển đúng cách gọi; mỗi lần gọi đi qua `policy.check` và ghi chi phí vào sổ cái theo đơn vị và cược. Mô hình AI trong `models.yaml` hiện nay trở thành một loại mục trong danh mục này.

### Một mục ghi gì

| Trường | Ý nghĩa | Ví dụ |
| --- | --- | --- |
| Năng lực | Việc công cụ làm, theo một từ điển chung | audio.tts |
| Cách gọi | api, cli, mcp, gui, người | cli |
| Nơi chạy | đám mây, máy anh, máy có card từ N GB | máy anh |
| Giá | theo lần, theo giây, theo credit, theo token, gói tháng | 0 USD |
| Giấy phép công cụ | tên giấy phép và link | Apache 2.0 |
| Quyền đầu ra | được bán, không được bán, có điều kiện | được bán |
| Gọi tự động | điều khoản có cho bot gọi không, giới hạn tốc độ | cho phép |
| Đầu ra do AI tạo | có hay không, để code tự gắn nhãn | có |
| Trạng thái | đề xuất, thử, đã duyệt, ngưng | thử |
| Nguồn và ngày kiểm | link trang giá, giấy phép, điều khoản; câu trích nguyên văn cho giấy phép, quyền đầu ra và gọi tự động; ngày mở | "dual-licensed under MIT or Apache-2.0", 16/10/2026 |

Từ điển năng lực bắt đầu với: `text.write`, `text.translate`, `image.generate`, `image.edit`, `vector.draw`, `layout.pdf`, `layout.slides`, `audio.tts`, `audio.music`, `audio.edit`, `video.generate`, `video.edit`, `video.subtitle`, `speech.transcribe`, `web.read`, `data.sheet`, `code.run`, `model3d`. Thêm năng lực mới là thêm một từ, không sửa code.

### Bộ chuyển theo cách gọi

| Cách gọi | Bộ chuyển làm gì | Độ tin cậy |
| --- | --- | --- |
| API | Gọi HTTP với khóa trong biến môi trường, cấp theo vai trò như `opco grant` hiện nay | Cao |
| CLI | Chạy lệnh trong một thư mục làm việc riêng, có hạn giờ; đầu vào và đầu ra là tệp | Cao |
| MCP | Gọi công cụ của một máy chủ MCP | Trung bình |
| GUI (điều khiển màn hình) | Điều khiển trình duyệt hoặc app. Chậm, dễ vỡ, tốn token. Bật riêng cho từng công cụ anh duyệt; Suno là công cụ đầu tiên. Mỗi lần chạy lưu ảnh chụp từng bước vào hồ sơ | Thấp |
| Người | Giao việc cho người làm thuê; luôn là nhóm C vì là thuê người | Tuỳ người |

### Chấm các công cụ anh gửi

| Công cụ | Làm gì | Cách gọi | Giá | Bán đầu ra | Vào dây chuyền tự động? |
| --- | --- | --- | --- | --- | --- |
| Các app Craft (PhotoCraft, FilmCraft, SoundCraft và 10 app khác) | Trình sửa ảnh, video, âm thanh, PDF, slide… chạy trên máy. Cả 13 trang em đã mở đều không nói tới AI tạo nội dung | Cả 13 app có CLI "for scripts and agents" nhưng trang chỉ có bản macOS. Trang của DesignCraft, FigCraft, WordCraft, DeckCraft, GridCraft có máy chủ MCP; repo PhotoCraft cũng có lệnh `photocraft-cli mcp` (Đ). Trang không nói MCP chạy trên hệ nào | Miễn phí, mã mở | PhotoCraft: MIT hoặc Apache-2.0 (Đ, repo). Các app còn lại: trang không nêu tên giấy phép, em chưa kiểm hết repo | Có, làm khâu sửa và ráp (anh chọn ngày 14/10). Trên macOS dùng CLI. Trên Windows hoặc Linux chưa chắc gọi được: MCP của PhotoCraft nằm trong CLI mà CLI chỉ có bản macOS. Đường còn lại là tự biên dịch CLI từ mã nguồn (README không ghi cách, em chưa thử) hoặc điều khiển màn hình |
| ArtCraft (app chính) | Tạo ảnh, video bằng nhiều mô hình AI | App máy tính; em chưa thấy cách gọi tự động phần tạo AI | 8 USD/tháng cho 1.000 credit | Tuỳ mô hình bên dưới | Không dùng (anh quyết ngày 14/10) |
| Suno | Sáng tác nhạc | Web, qua agent điều khiển trình duyệt (anh đang dùng). Không có API công khai | Anh đã có gói trả phí; em chưa biết giá gói để ghi vào lời lỗ | Gói trả phí: có, nếu tải qua nút tải chính thức và trong hạn mức tải. Quyền dùng bài đã tải còn sau khi hủy gói. Suno không bảo đảm bài có bản quyền; bản remix không được dùng thương mại (Đ) | Có, qua bộ chuyển điều khiển trình duyệt (anh chọn ngày 14/10). Vì Suno được khóa tài khoản và xóa tệp theo ý mình: tải bài về kho tài sản ngay sau khi tạo; trần số bài mỗi ngày dưới hạn mức tải; ACE-Step làm dự phòng; không bán quyền độc quyền nhạc AI |
| ACE-Step 1.5 | Nhạc có lời | Mô hình mở, chạy máy, dưới 4 GB card | 0 USD | MIT; trang ghi được dùng thương mại | Có. Dự phòng nếu tài khoản Suno gặp vấn đề |
| Kokoro | Giọng đọc | Mô hình mở | 0 USD | Apache 2.0 (T) | Không cho sản phẩm tiếng Việt: danh sách giọng chỉ có 9 ngôn ngữ, không có tiếng Việt (Đ). Dùng được cho sản phẩm tiếng Anh |
| XTTS-v2 | Giọng đọc, nhái giọng | Mô hình mở | 0 USD | Không thương mại (T) | Không, cho sản phẩm bán |
| Wan 2.2 | Tạo video | Mô hình mở. Bản 5B cần card từ 24 GB, bản A14B cần 80 GB (Đ) | 0 USD nếu có card; thuê RTX 4090 ở RunPod 0,89 USD/giờ, khoảng 35 giây video mỗi USD (S) | Apache 2.0 (Đ) | Chỉ khi máy anh có card từ 24 GB, hoặc anh duyệt thuê máy |
| ffmpeg | Cắt, ghép video, âm thanh, phụ đề | CLI | 0 USD | Không ảnh hưởng đầu ra (S) | Có. Khâu ráp cơ bản |
| Claude, GPT (đã có) | Viết, kiểm, nhìn ảnh | API | Theo token | Có | Có, đã có trong code |

Danh mục còn thiếu hai năng lực: image.generate (tạo ảnh AI, vì anh không dùng gói ArtCraft) và audio.tts tiếng Việt được bán. Công thức nào cần hai năng lực này thì chưa chạy được (xem cột "Chạy được hôm nay?" ở phần Công thức sản phẩm). Tìm hai công cụ này là việc đầu tiên của trinh sát công cụ; chuỗi đầu tiên không cần chúng.

### Trinh sát công cụ

Một vai trò agent mới tìm công cụ, mở trang giá, giấy phép và điều khoản, rồi điền sẵn một mục ở trạng thái "đề xuất". Mỗi dòng về giấy phép, quyền đầu ra và gọi tự động ghi câu trích nguyên văn, link và ngày mở, để anh duyệt từ câu gốc chứ không từ lời tóm tắt của agent (nhóm C). Sau khi duyệt, công cụ chạy thử trên một việc nhỏ và ghi chi phí thật, rồi mới vào công thức. Chạy thử chỉ cho biết công cụ chạy được, không cho biết có được bán đầu ra. Đây là cách công ty tự mở rộng mà không cần sửa code: công cụ có CLI hoặc API chỉ cần một mục dữ liệu và một mẫu lệnh.

## Công thức sản phẩm

Một sản phẩm là một công thức: chuỗi bước, mỗi bước gọi một năng lực chứ không gọi thẳng một công cụ. Công thức là dữ liệu, nên agent tự ghép công thức mới mà không cần sửa code. Dây chuyền viết bài hiện nay trở thành công thức đầu tiên.

| Sản phẩm | Năng lực cần | Công cụ có thể dùng | Chạy được hôm nay? |
| --- | --- | --- | --- |
| Ebook, sách bài tập PDF | text.write, layout.pdf; image.generate nếu cần ảnh | Claude, PdfCraft hoặc thư viện PDF của Python | Được nếu không cần ảnh AI. Đây là chuỗi đầu tiên |
| Video ngắn có lời đọc | text.write, audio.tts, image.generate hoặc video.generate, video.edit | Claude, Wan 2.2, ffmpeg hoặc FilmCraft; giọng đọc tiếng Việt chưa có | Chưa: thiếu giọng tiếng Việt được bán và ảnh AI; video AI cần card từ 24 GB |
| Nhạc nền bán quyền sử dụng | audio.music, audio.edit | Suno (qua agent), ACE-Step, SoundCraft | Làm ra được; chưa có kênh bán vì các chợ nhạc lớn không nhận nhạc AI (T) |
| Bộ mẫu thiết kế, slide | vector.draw, layout.slides | VectorCraft, DeckCraft | Được trên macOS qua CLI; hệ khác chưa chắc (xem dòng Craft ở Danh mục) |
| Phần mềm nhỏ | text.write, code.run | Đã có (biến thể phần mềm) | Được; anh để biến thể này đi sau cùng |

Ví dụ công thức video giải thích ngắn (minh họa, số liệu là ví dụ; công thức này chưa chạy được hôm nay vì thiếu hai công cụ ghi chú bên trong):

```yaml
recipe: video-giai-thich-ngan
version: 1
steps:
  - id: script
    capability: text.write
    checks: [sources_resolve, max_words: 220]
  - id: voice
    capability: audio.tts
    params: {lang: vi}                # chưa chạy được: chưa có công cụ giọng tiếng Việt được bán
    checks: [speech_matches_script]   # nghe lại bằng speech.transcribe, so với kịch bản
  - id: visuals
    capability: image.generate        # chưa có công cụ trong danh mục
    repeat: per_scene
  - id: assemble
    capability: video.edit            # ffmpeg hoặc FilmCraft CLI
    checks: [max_seconds: 60, subtitles_present]
  - id: label
    capability: video.edit
    action: add_ai_label              # code tự thêm nhãn nhìn thấy và siêu dữ liệu máy đọc khi có tệp do AI tạo
review: checker_vision                # người kiểm khác hãng xem khung hình và kịch bản
```

Quy tắc:

- Bộ chọn công cụ lấy công cụ đã duyệt, rẻ nhất, có quyền bán đầu ra và có máy chạy được. Đổi công cụ không phải sửa công thức.
- Trước khi chạy, code cộng giá dự kiến từ danh mục. Vượt ngân sách của đơn vị thì không chạy.
- Mỗi bước kiểm bằng máy trước, rẻ và chắc; người kiểm khác hãng xem ở cuối.
- Công thức mới: agent soạn, chạy thử ở chế độ nháp (không đăng, không bán), ghi chi phí thật, rồi gửi anh duyệt mẫu đầu tiên (nhóm C). Chạy lại công thức đã duyệt là nhóm B.
- Sửa công thức đã duyệt: đổi tham số là nhóm B; thêm bước hoặc năng lực mới là nhóm C.

## Nguồn gốc tệp và kiểm tra chất lượng media

Mọi tệp công ty làm ra là một tài sản có hồ sơ nguồn gốc. Nhờ hồ sơ này, code tự gắn nhãn AI, tự chặn đầu ra không được bán, tự soạn hồ sơ cho anh duyệt, và dùng lại tài sản cho sản phẩm khác.

| Trường trong hồ sơ tài sản | Dùng để |
| --- | --- |
| Mã băm, loại tệp, kích thước | Nhận diện, chống trùng |
| Công thức, bước, công cụ và phiên bản | Biết công cụ nào làm ra, từ đó biết quyền bán và có cần nhãn AI |
| Đầu vào: tài sản khác, link nguồn, prompt | Truy ngược; tệp lấy từ ngoài phải có giấy phép ghi kèm |
| Do AI tạo: có hay không | Gắn nhãn nhìn thấy được và ghi siêu dữ liệu máy đọc vào tệp (xem đoạn Nhãn AI bên dưới) |
| Quyền: được bán, không được bán, chờ kiểm | Chặn phát hành |
| Chi phí | Tính lời lỗ từng sản phẩm |
| Người duyệt, ngày, số phút duyệt | Hồ sơ; đo thời gian anh bỏ ra |

**Quy tắc quyền:** quyền của một sản phẩm bằng quyền hẹp nhất trong mọi tệp góp vào nó. Một tệp không được bán làm cả sản phẩm không được bán. Code kiểm điều này, không cần anh nhìn.

Nhãn AI: Luật Trí tuệ nhân tạo 134/2025 (hiệu lực 01/03/2026) giao việc đánh dấu âm thanh, hình ảnh, video do AI tạo "ở định dạng máy đọc" cho nhà cung cấp hệ thống AI (Điều 11.2). Công ty là bên triển khai: phải thông báo khi nội dung có thể gây nhầm lẫn (11.3), và gắn nhãn nội dung giả người thật hoặc tái tạo sự kiện thật (11.4) (Đ). Hình thức cụ thể do Chính phủ quy định (11.6); em chưa đọc Nghị định 142 mà bản phản biện dẫn, nên chưa biết hình thức đó. Code làm cả hai: nhãn nhìn thấy được trên sản phẩm, và siêu dữ liệu máy đọc trong tệp (ví dụ theo chuẩn C2PA), vì ghi siêu dữ liệu gần như không tốn gì \[S\].

**Kho tài sản:** tệp đã duyệt được dùng lại mà không tốn tiền tạo lại (ảnh nền, đoạn nhạc, giọng đọc, khuôn slide). Kho lớn dần là lợi thế cộng dồn của công ty \[S\].

### Kiểm tra chất lượng theo lớp

Mỗi lớp rẻ hơn lớp sau, nên lớp rẻ chạy trước và loại bớt việc cho lớp đắt.

| Lớp | Kiểm gì | Ví dụ |
| --- | --- | --- |
| 1. Máy, xác định | Tệp mở được, đúng định dạng, thời lượng, độ phân giải, độ lớn âm thanh, có phụ đề | Đọc thông số tệp bằng ffprobe |
| 2. Đối chiếu chéo | Nghe lại lời đọc, chuyển thành chữ rồi so với kịch bản; đọc chữ trong ảnh để bắt lỗi chính tả | speech.transcribe, đọc chữ trong ảnh |
| 3. Người kiểm khác hãng | Mô hình hãng khác xem khung hình, nghe đoạn mẫu, chấm theo tiêu chí | Hình méo, chữ sai, mặt người thật, logo, nội dung nhạy cảm |
| 4. Bộ lỗi mồi theo loại media | Mỗi loại media có ít nhất 20 mẫu có lỗi và 20 mẫu sạch. Người kiểm phải bắt ít nhất 80% lỗi và báo nhầm tối đa 20% | Giống rev 11, mở rộng sang ảnh, tiếng, video. Bộ chữ hiện chưa có mẫu sạch nên chưa đo được báo nhầm; phải thêm |
| 5. Anh xem mẫu | Cho tới khi loại media đó qua lớp 4 |  |

Loại media chưa qua lớp 4 thì mọi lần phát hành vẫn tới anh. Em chưa kiểm mô hình người kiểm hiện tại (GPT của OpenAI) có xem ảnh, nghe tiếng được không.

## Kênh, cách thu tiền và loại cược

Kênh và cách thu tiền cũng là mục trong danh mục, giống công cụ. Mỗi cược khai báo bốn thứ: công thức, kênh, cách thu tiền, loại cược. Loại cược quyết định cổng đo, nên cổng bán trước 14 ngày không còn áp cho mọi việc.

| Kênh | Cách nối | Điều đã biết | Cửa công ty không tự mở được | Ai mở |
| --- | --- | --- | --- | --- |
| Thư mục (đã có) | Ghi tệp | Chỉ để thử nội bộ | Không có | Không cần |
| Trang bán riêng, bản tin email | API của dịch vụ | Luật 122/2025 Điều 14.1: thông báo trước khi chạy trang bán có đặt hàng trực tuyến (Đ) | Thông báo theo Điều 14.1. Thu tiền VietQR bị khóa tới khi kế toán xác nhận ngưỡng thuế | Anh; anh và kế toán |
| Chợ sản phẩm số (Polar, chợ ảnh, nhạc, mẫu) | API hoặc anh tải lên tay | Polar xếp ebook, đặt trước và công cụ tạo nội dung AI vào nhóm hạn chế (Đ). Các chợ nhạc lớn không nhận nhạc AI (T) | Polar phải chấp nhận loại hàng bị hạn chế; chợ nhạc có thể không mở với nhạc AI | Nền tảng quyết; anh nộp hồ sơ |
| Mạng xã hội (YouTube, TikTok, Facebook) | Chỉ API chính thức | TikTok: ứng dụng chưa qua kiểm duyệt chỉ đăng được chế độ riêng tư, tối đa 5 người dùng mỗi 24 giờ; TikTok không duyệt "A utility tool to help upload contents to the account(s) you or your team manages" (Đ). YouTube: tải video qua API không còn bị giữ ở chế độ riêng tư; chỉ trả tiền khi kênh có 1.000 người đăng ký và 4.000 giờ xem trong 12 tháng, hoặc 1.000 người đăng ký và 10 triệu lượt xem Shorts trong 90 ngày (Đ) | TikTok: công cụ tự đăng cho tài khoản của mình không được duyệt, nên bài đăng qua API chỉ ở chế độ riêng tư. YouTube: ngưỡng kiếm tiền | TikTok: anh đăng tay. YouTube: khán giả thật, không ai mua được |

Cách thu tiền trong danh mục ban đầu: bán một lần, đăng ký theo tháng, chia doanh thu nền tảng, bán quyền sử dụng (ảnh, nhạc, mẫu), nhận đặt làm, tài trợ, tiếp thị liên kết. Cách mới thêm vào qua cổng duyệt của anh.

| Loại cược | Dùng khi | Cổng đo | Dừng khi |
| --- | --- | --- | --- |
| Bán trước (đã có) | Bán thẳng cho người mua | Ít nhất 3 người lạ trả tiền trong 14 ngày | Không đạt; hoàn tiền tự động |
| Xây khán giả (anh đã đồng ý) | Kênh mạng xã hội, bản tin | Giờ xem và người đăng ký mới của từng sản phẩm, tính trên mỗi USD (token cộng tiền mặt) và trên mỗi giờ anh duyệt, sau N tuần. Trong lúc đo chưa có doanh thu | Đường tăng phẳng sau N tuần |
| Doanh thu nền tảng | Kênh đã bật kiếm tiền | So doanh thu với chi phí theo nhóm sản phẩm cùng tuổi (ví dụ cùng 28 ngày sau khi đăng), vì doanh thu một video còn tích lũy nhiều tháng | K nhóm cùng tuổi liên tiếp lỗ |
| Bán quyền sử dụng | Ảnh, nhạc, mẫu trên chợ. Nhạc AI chỉ bán quyền không độc quyền, vì Suno không bảo đảm bài có bản quyền | Số lượt bán trên mỗi tài sản trong 30 ngày | Không bán được; tài sản vẫn nằm trong kho |
| Nhận đặt làm | Làm theo yêu cầu khách | Đơn trả tiền đầu tiên trong N ngày | Không có đơn |

N và K là số anh đặt trong hiến chương; em đề xuất số ban đầu ở phần "Anh cần quyết". Thước đo cũ "tăng trên mỗi USD" bị bỏ, vì công cụ chạy trên máy gần như không tốn tiền mặt nên phép chia cho ra số vô nghĩa. Giờ duyệt của anh lấy từ số phút ghi trong sổ cái mỗi lần duyệt.

## Kinh tế của công ty

Phần này không dự báo doanh thu. Nó mô tả công thức lợi nhuận của công ty, các cần gạt tối ưu chi phí, và các tham số mà code sẽ điều chỉnh. Số cụ thể sẽ là số liệu vận hành thật của công ty.

### Công thức lợi nhuận

Lợi nhuận kỳ = Σ biên đóng góp của các đơn vị − chi phí thăm dò − chi phí nền tảng − chi phí chuyên gia.

- **Biên đóng góp của một đơn vị** = doanh thu − phí thanh toán − token sản xuất và kiểm tra − chi công cụ (theo lần, phần chia gói tháng) − token hỗ trợ − hạ tầng riêng − hoàn tiền.
- **Chi phí thăm dò** = token cho thẻ cơ hội và bán thử của các cược chưa thành đơn vị. Đây là “chi phí R&D” của công ty tự hành.
- **Chi phí nền tảng** = hạ tầng chung, eval, kiểm toán, trinh sát môi trường. Gần như cố định.
- **Giờ của anh** không phải tiền mặt nhưng là chi phí kinh tế; một công ty “có lãi” nhưng cần 20 giờ/tuần của anh thì chưa tự hành. Nó được đo và báo cáo cùng lợi nhuận.

### Lời lỗ từng sản phẩm và tiền mặt cho công cụ

Công ty kiếm nhiều nhất khi từng sản phẩm có lãi và tiền dồn vào cái lãi nhất. Vì vậy mỗi sản phẩm có sổ lời lỗ riêng, và bộ phân bổ của rev 11 dùng lãi đã tính cả chi công cụ, không chỉ token.

```latex
\text{lãi mỗi sản phẩm} = \text{doanh thu} - (\text{token} + \text{công cụ theo lần} + \text{phần chia gói tháng} + \text{kiểm tra} + \text{phí kênh và thanh toán})
```

Ước tính chi phí một video 60 giây, chưa tính token viết kịch bản. Số này em suy ra từ giá RunPod và tốc độ trong README Wan 2.2 \[S\]; công ty thay bằng số đo thật sau lần chạy nháp đầu tiên:

| Cách làm | Thời gian máy | Tiền mặt |
| --- | --- | --- |
| Tạo trọn 60 giây bằng Wan 2.2 bản 5B trên RTX 4090 thuê (0,89 USD/giờ; 5 giây 720P mất dưới 9 phút) | Tối đa khoảng 108 phút | Tối đa khoảng 1,6 USD |
| Ráp từ ảnh tĩnh và giọng đọc chạy trên máy, bằng ffmpeg | Vài phút trên máy anh | Gần 0 USD, nhưng cần công cụ ảnh AI và giọng tiếng Việt chưa có |

Trần 1 USD mỗi ngày không đủ cho một video tạo trọn như vậy. Bộ chọn công cụ và ước tính trước khi chạy tồn tại để công ty tự thấy khoảng cách này và chọn cách rẻ hơn. Card RTX A5000 24 GB rẻ hơn (0,27 USD/giờ) nhưng em chưa biết tốc độ. Ví dụ cũ dựa trên gói ArtCraft đã bị bỏ vì anh không dùng gói đó.

Quy tắc tiền:

- Hai nguồn tiền tách riêng: token (khoảng 100 USD mỗi tháng khi thử) và tiền mặt (tối đa 50 USD trước khi có doanh thu).
- Giờ duyệt của anh là nguồn thứ ba và có hạn nhất: mỗi lần duyệt ghi số phút vào sổ cái, và lời lỗ từng sản phẩm báo kèm số phút đó.
- Thứ tự chọn công cụ: miễn phí chạy trên máy, rồi trả theo lần, cuối cùng mới tới gói tháng.
- Mỗi gói tháng là một quyết định nhóm C, kèm phép tính hòa vốn: cần bán bao nhiêu sản phẩm để trả gói. Chỉ đề xuất khi đã có một cược qua cổng.
- Gói Suno anh đang trả là chi phí cố định mỗi tháng, chia đều cho số bài đã tải trong tháng và cộng vào lời lỗ của sản phẩm dùng bài đó.
- Thuê máy có card theo giờ là tiền mặt trả theo lần, có trần mỗi ngày.
- Trần tiền mặt và trần token áp cho cả danh mục các công ty, không chỉ từng công ty, để chạy nhiều công ty không nhân trần lên. Trần token danh mục bật mặc định, không chờ biến môi trường như code hiện nay.
- Ước tính trước khi chạy, đo thật sau khi chạy. Lệch nhiều thì giá trong danh mục được cập nhật.

### Quy tắc ngân sách tự nuôi

Ngân sách token kỳ sau = B0 + α × biên đóng góp trung bình 3 kỳ gần nhất.

- B0 là sàn chủ sở hữu cấp. Anh đã chọn khoảng 100 USD/tháng cho giai đoạn thử nghiệm.
- α (0,3–0,5 \[GT\]) là phần lợi nhuận được tái đầu tư; phần còn lại là lợi nhuận giữ lại hoặc quỹ dự phòng.
- Ngân sách chia hai phần: **khai thác** (cho đơn vị đang có lãi, theo tỷ lệ biên đóng góp trên mỗi USD token) và **thăm dò** (tỷ lệ cố định, ví dụ 30% \[GT\], cho cược mới). Đây là bài toán khai thác và thăm dò kinh điển; giữ phần thăm dò cố định để công ty không chết cùng một sản phẩm đang suy \[LT\].
- Tiền mặt ngoài token chỉ chi từ doanh thu đã về, trừ khoản trần khoảng 50 USD trước doanh thu anh đã duyệt.

### Cần gạt tối ưu chi phí

Xếp theo tác động dự kiến \[S\]. Không cần gạt nào cắt kiểm tra hay kiểm soát, vì chi phí một lỗi lọt tới khách thường lớn hơn nhiều chi phí kiểm tra.

1. **Dừng cược sớm.** Chi phí lớn nhất của công ty tự hành là nuôi thứ không ai mua. Bán thử trước khi xây cắt phần lớn chi phí này.
2. **Định tuyến mô hình.** Mỗi vai trò bắt đầu với mô hình rẻ nhất qua được eval của vai trò đó; chỉ lên mô hình đắt khi việc thất bại. Giá hiện nay chênh nhau khoảng 40 lần giữa Haiku 5.5 và Opus 5.5 \[Đ\].
3. **Kiểm tra xác định trước, LLM sau.** Test, lược đồ, đối chiếu nguồn gần như miễn phí; chỉ dùng LLM kiểm phần còn lại.
4. **Dùng lại.** Thành phần code, mẫu nội dung, prompt đã qua eval được lưu trong nền tảng và dùng cho đơn vị sau. Bộ nhớ đệm prompt và gọi theo lô khi không cần nhanh.
5. **Mua thay vì xây** những thứ không tạo khác biệt (N12).
6. **Vai trò agent phải tự trả được tiền của nó.** Mỗi vai trò có sổ chi phí riêng; vai trò nào tốn token mà không cải thiện chỉ số nào đo được thì bị đề xuất gộp hoặc bỏ.

### Chỉ số của công ty

| Chỉ số | Đo gì | Vì sao quan trọng |
| --- | --- | --- |
| Biên đóng góp sau token | Lãi thật của từng đơn vị | Quyết định phân bổ |
| Biên đóng góp trên mỗi USD token | Hiệu suất dùng AI | So sánh đơn vị, so sánh mô hình |
| Tỷ lệ cược thành đơn vị | Khả năng tự tìm ngách | Đây là năng lực cốt lõi của công ty tự hành |
| Chi phí tìm được một đơn vị có lãi | Tổng chi thăm dò / số đơn vị mới | Độ đắt của việc tăng trưởng |
| Tỷ lệ can thiệp của người | Giờ của anh và số việc nhóm C trên mỗi triệu doanh thu | Đo mức tự hành thật; cũng là số liệu chính khi bán mô hình |
| Tỷ lệ lỗi lọt tới khách | Lỗi khách phát hiện trên tổng giao hàng | Chất lượng thật |
| Tỷ lệ bắt lỗi mồi | Người kiểm bắt được bao nhiêu lỗi cài sẵn | Chất lượng của bước kiểm tra |
| Số tháng còn sống | Quỹ / mức đốt ròng | Sống sót |
| Số phút anh duyệt mỗi sản phẩm | Ghi vào sổ cái mỗi lần duyệt | Đo nút thắt ở người; thước đo cho mục tiêu giảm duyệt |

Với mục tiêu sau này gọi vốn hoặc bán mô hình công ty số, ba chỉ số có giá trị nhất là: tỷ lệ can thiệp của người, chi phí tìm được một đơn vị có lãi, và biên đóng góp trên mỗi USD token \[S\]. Cả ba đều phải được ghi từ ngày đầu, vì nhà đầu tư tin số liệu lịch sử hơn số liệu tự báo cáo (E10).

## Biến thể: phần mềm và nội dung số

Hai biến thể dùng chung toàn bộ hệ 2, 3, 3\*, 4, 5 và nền tảng. Từ rev 12, chúng không còn là hai nhánh code mà là hai bộ công thức và hai hiến chương mẫu; một công ty có thể chạy công thức của cả hai. Chúng khác nhau ở hệ 1: dây chuyền sản xuất, cách kiểm tra chất lượng, cách kiếm tiền và rủi ro riêng.

| Khía cạnh | Công ty sản xuất phần mềm | Công ty sản xuất nội dung số |
| --- | --- | --- |
| Thứ đơn vị bán | SaaS nhỏ, công cụ, plugin, mẫu code, gói việc phần mềm giá cố định | Ebook, khóa học ngắn, mẫu, bản tin trả phí, gói nội dung cho doanh nghiệp |
| Dây chuyền | Đặc tả → chia việc → viết code → test → review khác hãng → triển khai có hoàn tác → giám sát → hỗ trợ | Chọn đề → thu và lưu nguồn → dàn ý → viết → đối chiếu nguồn → biên tập khác hãng → gắn nhãn AI → đăng → phân phối |
| Kiểm tra chất lượng | Phần lớn xác định: test, kiểm tra kiểu, quét bảo mật. Rẻ và đáng tin | Phần lớn cần đánh giá: đúng sự thật, giọng văn, hữu ích. Đối chiếu nguồn làm được, phần còn lại cần LLM và lấy mẫu người |
| Độ tin cậy của agent hiện nay | Cao ở việc ngắn có test; thấp ở việc dài (E1, E5) | Viết nhanh và rẻ; dễ bịa nguồn nếu không buộc đối chiếu \[S\] |
| Kênh bán | Marketplace (plugin, template), cộng đồng lập trình, bán toàn cầu qua bên bán hộ | Email, mạng xã hội, cộng đồng; Việt Nam trước qua VietQR, ví điện tử |
| Không dựa vào | — | Quảng cáo hiển thị và traffic tìm kiếm: RPM Việt Nam rất thấp, Google giảm lượt về, chính sách spam \[Đ\] |
| Rủi ro riêng | Quyền production, dữ liệu khách, bảo mật phụ thuộc, lỗi làm hỏng dữ liệu | Sai sự thật, bản quyền, phỉ báng, nhãn AI cho ảnh, âm thanh, video, chính sách nội dung hàng loạt |
| Luật cần nhúng vào dây chuyền | Bảo vệ dữ liệu cá nhân, chuyển dữ liệu xuyên biên giới, bảo vệ người tiêu dùng (gia hạn, hủy) | Luật AI (nhãn), sở hữu trí tuệ, quảng cáo, bảo vệ người tiêu dùng |
| Chi phí bảo trì sau khi bán | Có: mỗi sản phẩm cần giám sát và sửa lỗi | Thấp với sản phẩm một lần; có với bản tin định kỳ |

**Thứ tự đã quyết (13/10):** nội dung số trước, phần mềm sau cùng. Rev 11 từng đề xuất phần mềm trước vì kiểm tra bằng test rẻ và đáng tin; anh chọn nội dung trước.

## Chuỗi đầu tiên tới đồng tiền đầu tiên

**Đề xuất, chờ anh xác nhận.** Công ty chạy trọn một chuỗi cho tới khi một người lạ trả tiền, rồi mới mở sang loại sản phẩm khác. Cách này giữ nguyên tắc "mỗi lần một hướng" của rev 11 và vừa ngân sách token khoảng 100 USD mỗi tháng khi thử. Chủ đề cụ thể vẫn do công ty tự tìm, em không chọn trước.

| Khâu | Chọn | Cửa còn đóng | Ai mở |
| --- | --- | --- | --- |
| Sản phẩm | Sản phẩm chữ hoặc PDF tiếng Việt, không dùng ảnh AI | Chưa biết có người cần không | Trinh sát tìm nhu cầu, rồi cổng bán trước |
| Công cụ | Claude viết; người kiểm khác hãng; dàn trang bằng PdfCraft hoặc DesignCraft nếu máy anh là macOS, không thì bằng thư viện PDF của Python | Không có | Không cần |
| Kênh | Trang bán của công ty; chưa chọn nơi đặt trang | Thông báo theo Luật 122/2025 Điều 14.1 trước khi chạy trang bán có đặt hàng trực tuyến | Anh |
| Thu tiền | VietQR vào tài khoản cá nhân (anh đã quyết ngày 14/10) | Code khóa thu tiền thật cho tới khi kế toán xác nhận ngưỡng thuế | Anh và kế toán |
| Cổng | Bán trước: 3 người lạ trả tiền trong 14 ngày | Không có | Không cần |

Vì sao chuỗi này: mọi công cụ đã có sẵn hoặc miễn phí, và mọi cửa còn đóng là cửa anh mở được. Rủi ro chính là chưa biết có người cần, nên cổng bán trước đứng trước khi làm nhiều.

Có điều kiện, đi sau chuỗi này:

- Video AI chỉ khi máy anh có card từ 24 GB VRAM, hoặc anh đồng ý thuê máy theo giờ.
- Nhạc Suno là cược khán giả; YouTube chỉ trả tiền sau khi kênh đạt ngưỡng của chương trình đối tác.

Xong khi: sổ cái có một khoản tiền thật của người lạ, kèm lời lỗ của sản phẩm đó và số phút anh đã bỏ ra duyệt.

## Đặc tả để lập trình

Phần này là cầu nối sang source code: cần những module nào, dữ liệu nào, sự kiện nào, cấu hình nào, và xây theo thứ tự nào. Công nghệ đã chọn ngày 13/10: Python và PostgreSQL, chạy trên máy anh trước rồi lên VPS. Code nằm ở [Company-OS](https://github.com/nqthiep/Company-OS).

### Module

| Module | Hệ | Trách nhiệm | Agent hay code | Rev 12 thêm |
| --- | --- | --- | --- | --- |
| Hiến chương | 5 | Đọc, kiểm tra hợp lệ, lưu phiên bản tệp cấu hình công ty; chỉ anh ghi được | Code | Danh sách chặn, loại cược, trần tiền mặt cho công cụ, trần cho cả danh mục, việc nhóm C mới; mục tiêu thôi gắn với bản tin ngách |
| Policy engine | 5, 3 | Trước mọi hành động: xác định nhóm A–D, kiểm trần, cho phép hoặc chặn hoặc đưa vào hàng duyệt | Code | Kiểm công cụ đã duyệt chưa, quyền đầu ra, điều khoản gọi tự động, danh sách chặn; trần token danh mục bật mặc định |
| Hàng duyệt | 5 | Việc nhóm C kèm hồ sơ; anh duyệt hoặc từ chối; có hạn chờ | Code + agent soạn hồ sơ | Gắn mã băm hồ sơ: hồ sơ sửa sau khi duyệt thì duyệt lại; ghi số phút duyệt |
| Sổ cái | Nền tảng | Ghi mọi khoản thu, chi, token theo đơn vị và vai trò; chỉ thêm, không sửa | Code | Chi công cụ (credit, giây, phần gói tháng, kể cả gói Suno); tách tiền mặt với token; số phút anh duyệt |
| Bộ phân bổ | 3 | Tính ngân sách mỗi kỳ theo quy tắc; mở, dừng cược theo cổng | Code | Dùng lãi đã tính chi công cụ; mỗi loại cược một hàm cổng |
| Bộ điều phối | 2 | Hàng đợi việc, chạy luồng, khóa tài nguyên, thử lại, ngắt vòng lặp | Code | Hàng đợi theo nơi chạy (máy có card, đám mây); mỗi cược phải khai đủ công thức, kênh, cách thu tiền, loại cược, lĩnh vực |
| Agent runtime | 1, 4 | Chạy một vai trò: nạp mô tả vai trò, chọn mô hình, cấp công cụ và khóa theo quyền, đếm token | Code bao quanh agent | |
| Danh mục công cụ | Nền tảng | Mục công cụ có phiên bản, chỉ thêm; mỗi dòng giấy phép kèm câu trích nguyên văn; models.yaml gộp vào | Code + trinh sát công cụ | Mới |
| Bộ chuyển | Nền tảng | Gọi công cụ qua API, CLI, MCP; điều khiển trình duyệt bật riêng cho công cụ anh duyệt (Suno) | Code | Mới |
| Bộ chọn công cụ | 2 | Chọn công cụ rẻ nhất hợp lệ cho một năng lực | Code | Mới |
| Kho tài sản | Nền tảng | Bảng tài sản chỉ thêm, có hồ sơ nguồn gốc; thư mục tệp trên máy; bài Suno tải về ngay sau khi tạo | Code | Mới |
| Bộ kiểm tra | 1 | Chạy bài nghiệm thu xác định, rồi người kiểm bằng LLM khác hãng | Code + agent | Lớp 1–3 cho ảnh, tiếng, video |
| Eval harness | 3\*, 4 | Bộ bài kiểm tra có lỗi mồi; chạy khi đổi mô hình, prompt, quy trình; lấy mẫu ngẫu nhiên | Code + agent kiểm toán | Bộ lỗi mồi theo loại media, mỗi loại ít nhất 20 mẫu lỗi và 20 mẫu sạch; bộ chữ thêm mẫu sạch và một test chạy người kiểm bằng mô hình thật |
| Bộ nhớ tổ chức | Nền tảng | Quyết định, chính sách đã duyệt, bài học; có hạn dùng và người duyệt | Code | |
| Danh tính và khóa | Nền tảng | Mỗi vai trò, mỗi đơn vị một khóa có trần chi; thu hồi khóa = nút dừng | Code | |
| Nhật ký và bảng ngoại lệ | 3 | Ghi mọi hành động; tính chỉ số; phát cảnh báo theo ngưỡng | Code + agent tóm tắt | |
| Cổng kết nối và kênh | Nền tảng | Bên bán hộ, ngân hàng, git, hosting, CMS, email, nhà cung cấp mô hình | Code | Kênh thành mục danh mục; chỉ dùng API chính thức |
| Dây chuyền sản xuất | 1 | Công thức là dữ liệu; viết bài, PDF và phần mềm là các công thức | Cấu hình + agent | Thay cho "mỗi biến thể một bộ luồng" |
| Trinh sát | 4 | Trinh sát thị trường, môi trường và công cụ | Agent | Trinh sát cơ hội học từ kết quả thật theo cặp công thức và kênh; prompt thôi ghi cứng "a software company" |
| web.fetch\_text, kiểm link | Nền tảng | Tải trang, kiểm link trong sản phẩm | Code | Kiểm robots.txt; chặn địa chỉ nội bộ và localhost; tách "bị chặn" khỏi "không tồn tại" |

### Thực thể dữ liệu chính

- **Charter** (hiến chương): phiên bản, mục tiêu, trần, danh mục cấm, bảng quyền, ngưỡng.
- **Opportunity** (thẻ cơ hội): vấn đề, người trả tiền, tín hiệu, điểm, loại rủi ro.
- **Bet** (cược): giả thuyết, ngân sách, cổng, hạn, trạng thái.
- **Unit** (đơn vị): cấu hình, dây chuyền, sổ lãi lỗ, khóa.
- **RoleSpec** (mô tả vai trò): đầu vào, đầu ra, công cụ, quyền, ngân sách, mô hình, bài nghiệm thu.
- **Task** (việc), **Artifact** (sản phẩm của việc), **Verdict** (kết quả kiểm tra).
- **Decision** (quyết định nhóm C: hồ sơ, người duyệt, kết quả).
- **LedgerEntry** (bút toán), **Metric** (chỉ số), **Incident** (sự cố), **MemoryItem** (mục bộ nhớ).
- **Tool** (mục danh mục công cụ), **Recipe** (công thức), **Asset** (tài sản có hồ sơ nguồn gốc), **Channel** (kênh và cách thu tiền).
- **Customer**: chỉ lưu tối thiểu để giao hàng và hỗ trợ; chỉ gửi vào mô hình bản đã ẩn danh (bỏ tên, email, điện thoại, địa chỉ, số giấy tờ), theo quyết định ngày 14/10.

### Sự kiện

`opportunity.scored` · `bet.opened` · `bet.gate_reached` · `bet.killed` · `unit.created` · `task.created` · `task.completed` · `verify.failed` · `verify.passed` · `release.deployed` · `release.rolled_back` · `payment.received` · `refund.requested` · `ticket.opened` · `budget.threshold` · `runway.low` · `audit.seeded_error_missed` · `environment.changed` · `approval.requested` · `approval.decided` · `incident.opened` · `charter.updated`

### Hiến chương mẫu

Ví dụ hình dạng tệp cấu hình. Số là giá trị khởi điểm \[GT\]; số có dấu `?` là chỗ anh đặt.

```yaml
charter_version: 1
owner: thiep
goal: "Lợi nhuận từ mọi việc pháp luật và đạo đức cho phép"
budget:
  base_monthly_usd: 100      # B0
  reinvest_ratio: 0.4        # alpha
  explore_share: 0.3
  daily_cap_ratio: 0.05
  cash_before_revenue_usd: 50
  tool_cash_daily_cap_usd: ?             # thuê máy, credit trả theo lần, mỗi công ty
  portfolio_tool_cash_daily_cap_usd: ?   # cộng mọi công ty
  portfolio_token_cap_usd: ?             # bật mặc định, không chờ biến môi trường
gates:
  unit_min_positive_months: 2
  pivots_allowed: 1
bet_types:
  presell:    {days: 14, min_payments: 3}
  audience:   {weeks: ?, measure: [watch_hours, new_subscribers], per: [product, usd, owner_hour]}
  platform:   {cohort_age_days: 28, max_losing_cohorts: ?}
  license:    {days: 30, min_sales_per_asset: ?, exclusive_ai_music: false}
  commission: {days: ?, min_paid_orders: 1}
forbidden_domains: [high_risk_ai, health, personal_finance, personal_legal, children, sensitive_data]
deny:
  - sell_noncommercial_output
  - automate_tool_that_forbids_bots
  - real_person_voice_or_face_without_consent
  - copyrighted_input_without_license
  - fake_engagement
  - multi_account_evasion
  - fetch_disallowed_by_robots
decision_classes:
  C: [new_vendor, first_launch, price_model_change, prod_migration, new_personal_data, legal_reply, charter_change,
      new_tool, new_recipe, new_channel, new_revenue_model, new_market, subscription, public_release, hire_human]
  D: [sign_contract, file_tax, move_money_out, open_account, impersonate_human, fake_review]
thresholds:
  seeded_error_catch_min: 0.8
  seeded_false_alarm_max: 0.2
  refund_rate_max: 0.10
  approval_queue_max: 5
  runway_months_min: 3
```

Rev 11 ghi `presell_days` và `presell_min_payments` trong `gates`; từ rev 12 hai số này nằm trong `bet_types.presell`.

### Thứ tự xây

Thứ tự xây của rev 11 (lõi không agent, rồi các bước sửa theo phản biện 13/10) đã gộp vào nhánh main của Company-OS ngày 14/10 (PR #1, commit c6bf3df): hiến chương nội dung, vòng chạy `opco cycle`, sinh worker, bảng của chủ, ẩn danh tin nhắn khách, VietQR khóa chờ kế toán; 100 test qua với mô hình giả. Chưa có lần gọi mô hình thật nào vì chưa có khóa API. Phần còn lại xây theo hai giai đoạn dưới đây.

Hai giai đoạn. Giai đoạn A chỉ làm đủ cho chuỗi đầu tiên; giai đoạn B làm phần tổng quát sau khi chuỗi đầu tiên có người lạ trả tiền. Mỗi bước một PR, có cách kiểm xong. Test chạy được không cần khóa API thật (dùng mô hình giả như hiện nay); ra sản phẩm thật vẫn cần khóa.

### Giai đoạn A: chuỗi đầu tiên

1. **Sửa nền đang có.** Kiểm link chặn địa chỉ nội bộ và kiểm robots.txt; prompt trinh sát và content.yaml thôi ghi cứng; vòng chạy thôi tự mở cược rỗng; bộ lỗi mồi chữ có mẫu sạch; trần danh mục bật mặc định.
   - Kiểm xong: mỗi lỗ có test; 100 test cũ vẫn qua.
2. **Công thức PDF tiếng Việt.** Viết, kiểm khác hãng, dàn trang PDF; tệp có hồ sơ nguồn gốc; ghi số phút anh duyệt.
   - Kiểm xong: chạy nháp ra một PDF có hồ sơ nguồn gốc và chi phí thật.
3. **Trang bán và VietQR.** Trang bán của công ty; thu tiền vẫn khóa cho tới khi kế toán xác nhận.
   - Kiểm xong: anh mở khóa sau khi đã thông báo theo Điều 14.1 và có xác nhận của kế toán.
4. **Chạy cổng bán trước.**
   - Kiểm xong: sổ cái có tiền thật của người lạ, lời lỗ sản phẩm và số phút anh duyệt.

### Giai đoạn B: phần tổng quát

1. **Nền công cụ.** Danh mục công cụ, bộ chuyển API và CLI, kho tài sản, chi công cụ vào sổ cái.
   - Kiểm xong: một lệnh ffmpeg thật chạy qua bộ chuyển, ghi tài sản và chi phí; policy chặn công cụ chưa duyệt và đầu ra không thương mại.
2. **Công thức là dữ liệu.** Chuyển viết bài, PDF và phần mềm thành công thức; thêm một công thức không phải chữ chỉ dùng công cụ miễn phí.
   - Kiểm xong: test cũ vẫn qua; công thức mới chạy nháp ra tệp có nhãn AI, siêu dữ liệu và hồ sơ nguồn gốc.
3. **Kiểm tra media.** Lớp 1 và 2, bộ lỗi mồi cho loại media đầu tiên (ít nhất 20 mẫu lỗi, 20 mẫu sạch).
   - Kiểm xong: bắt ít nhất 80% lỗi mồi, báo nhầm tối đa 20%.
4. **Loại cược và lời lỗ từng sản phẩm.** Cổng cho xây khán giả, doanh thu nền tảng, bán quyền sử dụng, nhận đặt làm; bộ phân bổ dùng lãi đã tính chi công cụ.
   - Kiểm xong: test cho từng cổng; trong test, bộ phân bổ dồn ngân sách vào cược lãi.
5. **Trinh sát công cụ và hồ sơ duyệt.** Agent đề xuất công cụ kèm câu trích giấy phép; việc đầu tiên là tìm công cụ ảnh AI và giọng tiếng Việt được bán.
   - Kiểm xong: anh duyệt hoặc từ chối một công cụ đề xuất ngay trên bảng.
6. **Bộ chuyển trình duyệt cho Suno.**
   - Kiểm xong: một bài tải về kho, có hồ sơ nguồn gốc và ảnh chụp từng bước.
7. **Kênh thật.** Bộ nối API chính thức cho kênh anh chọn.
   - Kiểm xong: một lần đăng thật đã qua duyệt.

Có điều kiện: video AI chỉ làm khi máy anh có card từ 24 GB hoặc anh duyệt thuê máy; thử biên dịch CLI của app Craft từ mã nguồn nếu máy anh không phải macOS.

## Rủi ro của mô hình

### Rủi ro của chính mô hình

- **Công ty tự tìm ngách là giả định rủi ro nhất.** Chưa có bằng chứng công khai về một công ty agent tự tìm được ngách có lãi bền vững mà không có người hiểu ngách đứng sau \[S\]. Thiết kế này không giả định agent đoán giỏi; nó dùng tiền thật làm trọng tài. Nếu phân phối không tốn tiền không mang được khách tới, cơ chế vẫn đúng nhưng sẽ chỉ báo “dừng” liên tục.
- **Người kiểm cùng mắc lỗi với người làm.** Lỗi giữa các mô hình có tương quan (E3). Lỗi mồi đo được lỗi đã biết, không đo được lỗi chưa ai nghĩ ra.
- **Luật chơi sai thì tự hành sai một cách đều đặn.** Ngưỡng trong hiến chương ban đầu là đoán; công ty sẽ tối ưu theo chúng cho đến khi anh sửa.
- **Anh là điểm hỏng duy nhất của hệ 5.** Khi anh vắng, công ty chỉ giữ nguyên trạng (chế độ an toàn), không phát triển.
- **Tiêm prompt và quyền quá rộng** vẫn là rủi ro bảo mật lớn nhất (E7, E9). Kiểm soát bằng kiến trúc quyền, không bằng prompt.
- **Rủi ro pháp lý** đã liệt kê ở sổ nghĩa vụ của báo cáo trước vẫn giữ nguyên; quan trọng nhất là chuyển dữ liệu cá nhân xuyên biên giới và nhãn nội dung AI.

### Điều gì chứng minh mô hình sai

| Nếu thấy | Thì |
| --- | --- |
| Tỷ lệ cược thành đơn vị bằng 0 sau khoảng 10 cược | Cơ chế tìm ngách cần thêm nguồn lợi thế: kênh riêng, người hiểu ngách, hoặc ngân sách quảng cáo nhỏ |
| Tỷ lệ can thiệp của người không giảm sau 3 tháng | Thiết kế quyền quá bảo thủ, hoặc agent chưa đủ năng lực cho loại việc đó; xem từng loại việc |
| Biên đóng góp trên mỗi USD token không tăng | Agent đang làm việc tốn hơn giá trị; bỏ bước hoặc đổi mô hình |
| Nghiên cứu mới cho thấy agent tự điều hành kinh doanh dài hạn tốt | Cho agent đảm nhận thêm phần hệ 3 và hệ 4; hệ 5 vẫn là người vì luật |

## Đã quyết

Các câu hỏi cuối rev 11 và một số câu của rev 12 đã có câu trả lời của anh, theo ghi chép của dự án:

| Ngày | Quyết định |
| --- | --- |
| 13/10 | Biến thể nội dung số trước; phần mềm sau cùng |
| 13/10 | Python và PostgreSQL; chạy trên máy anh trước, sau đó lên VPS. Mặc định dùng mô hình rẻ, leo thang lên mô hình giỏi hơn khi bí; người kiểm dùng mô hình hãng khác |
| 13/10 | Danh sách nhóm C giữ như hiến chương mẫu; nhóm D (hợp đồng, thuế) ở anh và kế toán |
| 13/10 | Công ty tự sinh worker số; thuê người ngoài luôn là nhóm C |
| 13/10 | Chạy nhiều công ty từ đầu, mỗi công ty một schema `co_<tên>` |
| 14/10 | Thu tiền nội dung ở Việt Nam bằng VietQR vào tài khoản cá nhân và tự kê khai; thu tiền thật khóa cho tới khi kế toán xác nhận ngưỡng thuế |
| 14/10 | Hỗ trợ khách chỉ nhận ticket đã ẩn danh; hồ sơ chuyển dữ liệu xuyên biên giới chỉ lập khi thật sự cần dữ liệu thật |
| 14/10 | Công ty linh hoạt, mục tiêu lớn nhất là lợi nhuận, làm mọi việc pháp luật và đạo đức cho phép; dùng mọi công cụ để làm mọi sản phẩm. Anh là người duyệt cổng pháp lý. Loại cược xây khán giả được duyệt |
| 14/10 | Dùng Suno gói trả phí qua agent điều khiển web; dùng 13 app Craft, không dùng gói ArtCraft |

## Anh cần quyết

Câu 1 và 2 quyết định giai đoạn A bắt đầu được hay chưa; các câu còn lại chờ được.

| # | Câu hỏi | Em đề xuất |
| --- | --- | --- |
| 1 | Duyệt hướng rev 12, với chuỗi đầu tiên là sản phẩm chữ hoặc PDF tiếng Việt bán qua VietQR? | Duyệt, rồi bắt đầu giai đoạn A |
| 2 | Máy chạy CompanyOS dùng macOS, Windows hay Linux; card đồ họa bao nhiêu GB VRAM? | Quyết định cách gọi app Craft (CLI chỉ có bản macOS) và video AI có chạy trên máy anh được không (cần từ 24 GB) |
| 3 | Agent anh dùng để điều khiển web Suno là agent nào, chạy ở đâu? | CompanyOS gọi agent đó nếu nó nhận lệnh từ dòng lệnh; nếu không, em làm bộ chuyển trình duyệt riêng chạy trên máy anh |
| 4 | Gói Suno của anh giá bao nhiêu mỗi tháng, hạn mức tải bao nhiêu bài? | Cần để ghi vào lời lỗ và đặt trần số bài mỗi ngày |
| 5 | Công cụ tạo ảnh AI và giọng đọc tiếng Việt | Để trinh sát công cụ đề xuất ở giai đoạn B: ưu tiên mô hình mở chạy máy, rồi API trả theo lần. Chuỗi đầu tiên không cần |
| 6 | Phát hành công khai: duyệt từng lần hay theo mẫu; mục tiêu giảm số phút duyệt là bao nhiêu, tới ngày nào? | Từng lần lúc đầu. Đặt mục tiêu sau khi có số phút đo thật của 10 lần duyệt đầu \[S\] |
| 7 | Số cho cổng và trần | Xây khán giả đo sau 8 tuần, theo giờ xem và người đăng ký mới trên mỗi USD và mỗi giờ duyệt; doanh thu nền tảng dừng khi 3 nhóm 28 ngày liên tiếp lỗ; tiền mặt cho công cụ tối đa 1 USD mỗi ngày cho cả danh mục, nằm trong trần 50 USD. Số khởi đầu em chọn, chưa dựa trên số liệu \[S\] |
| 8 | Danh sách chặn 7 mục ở phần Thay đổi | Đồng ý; anh thêm hoặc bỏ mục nào tùy anh |

## Lịch sử sửa đổi

### Rev 11, cập nhật v2 (13/10): phản biện và thay đổi thiết kế

Ngày 13/10, một thread phản biện đã đọc thiết kế và mã Company-OS, rồi kiểm từng điểm. **Đ** = đã đối chiếu code, nguồn hoặc chạy thử. **S** = suy luận. **Chưa kiểm** = chưa xác minh, không dùng làm căn cứ.

- **Không có vòng chạy (Đ, đúng).** Trinh sát, mở cược, cổng, sản xuất và đăng bài chỉ được gọi trong test. Bộ phân bổ không được gọi ở đâu. Thay đổi: thêm vòng V3 chạy theo lịch, nối trinh sát → cược → cổng, có trần số lượt gọi. Chưa có vòng này thì công ty chưa tự hành.
- **Không có cơ chế phân phối (Đ, đúng).** Vai trò “Người phân phối” chỉ có trong bảng, không có ngân sách, kênh hay thước đo. Thay đổi: thêm đại lượng “số người xem trang trong 14 ngày” vào cổng cược. Cược không có người xem bị dừng vì lý do phân phối, không phải vì bán kém.
- **Polar và đặt trước (Đ, đúng một phần).** Polar xếp “Pre-orders and paid waitlists”, “eBooks” và “AI content generation tools” vào nhóm “restricted”. Code \<code>bets.kill()\</code> không hoàn tiền. Paddle: chưa kiểm. Thay đổi: dừng cược phải hoàn tiền tự động; đặt trước chỉ dùng sau khi có bản dùng thử; chọn nhà bán hộ sau khi đối chiếu danh sách cấm của họ.
- **Chọn nội dung trước nhưng code vẫn là phần mềm (Đ, đúng).** Chưa có hiến chương nội dung hay kênh tiếng Việt; \<code>content.publish()\</code> không đăng ở đâu; kiểm nguồn chỉ tìm chữ “http”. Thay đổi: biến thể nội dung là đường đầu tiên, có hiến chương, bộ mồi nội dung và kiểm link bằng cách tải thật.
- **Thước đo sai (Đ, đúng).** Bộ mồi chỉ có ca có lỗi, nên người kiểm luôn FAIL vẫn đạt 100%. Chi token không gắn đơn vị hay cược. Thay đổi: thêm ca sạch vào bộ eval và đo tỷ lệ báo nhầm; gắn mọi chi token với đơn vị hoặc cược.
- **Lỗi cài đặt và lỗ hổng code (Đ, đúng).** Lệnh cài README thất bại (pip báo nhiều gói cấp cao nhất). Phê duyệt nhóm C dùng lại được vì chỉ so loại hành động, không so nội dung. \<code>TRUNCATE ledger\</code> xóa sạch sổ cái vì trigger chỉ chặn UPDATE và DELETE. Nút dừng chỉ thu hồi quyền trong cơ sở dữ liệu, khóa API vẫn còn hiệu lực. Trinh sát không truy cập web, nên chỉ dựa vào trí nhớ mô hình. Cổng cược đếm giao dịch, không kiểm người mua có phải người lạ.
- **Mỗi công ty có 100 USD riêng, không có trần chung (Đ, đúng).** Thay đổi: thêm trần chi cho cả danh mục.
- **Dữ liệu cá nhân (Đ, mâu thuẫn trong thiết kế).** Mục Customer nói dữ liệu cá nhân không được gửi vào mô hình, nhưng vai trò Hỗ trợ khách xử lý ticket chứa dữ liệu cá nhân. Thay đổi: chọn một trong hai, ẩn danh hóa trước khi gửi, hoặc lập hồ sơ chuyển dữ liệu xuyên biên giới. Quyết định này chờ luật sư.
- **Pháp lý (Chưa kiểm).** Ngưỡng 500 triệu đồng có thể đã đổi theo nghị định mới; cần kế toán xác nhận. Luật Thương mại điện tử 122/2025 có thể yêu cầu thông báo trước khi mở trang bán; cần kiểm trước khi mở. Giá Haiku 5.5 có bậc cao hơn cho prompt trên 100.000 token; models.yaml đang giả định chỉ có một bậc.
- **Hệ 4 (thích nghi) (S, một phần).** Thiết kế mô tả trinh sát tự hiệu chỉnh theo kết quả thật, nhưng code chưa làm điều đó. Ghi nhận: hiện chưa có.

So với v1, thứ tự xây được sửa ở mục Thứ tự xây. Các thay đổi mã nguồn tương ứng chưa làm, chờ anh duyệt.

### Rev 12, sửa ngày 16/10 theo phản biện

Bản này sửa theo [phản biện độc lập rev 12](https://claude.ai/artifact/7mVWcHeaiD8ARgwSzbcDqD). Em mở lại nguồn gốc của từng điểm trước khi sửa. Em đồng ý cả 18 điểm; riêng điểm 8, em sửa theo cách phản biện đề xuất nhưng với lý do pháp lý khác.

| # | Điểm phản biện | Em kiểm lại | Kết luận | Sửa ở phần |
| --- | --- | --- | --- | --- |
| 1 | Kênh nào cũng có một cửa công ty không tự mở được; chưa có đường nào đi trọn tới tiền | Đúng với TikTok, YouTube, Polar, VietQR (Đ); chợ nhạc là nguồn thứ cấp. Riêng YouTube: tải video qua API không còn bị giữ ở chế độ riêng tư (Đ), cửa là ngưỡng kiếm tiền | Đồng ý | Chuỗi đầu tiên; Kênh |
| 2 | Thiếu công cụ ảnh AI và giọng tiếng Việt được bán | Đúng: VOICES.md của Kokoro có 9 ngôn ngữ, không có tiếng Việt (Đ) | Đồng ý | Danh mục; Công thức |
| 3 | Số phần cứng Wan 2.2 sai | Đúng: README ghi 5B cần ít nhất 24 GB, A14B cần 80 GB khi chạy một card (Đ) | Đồng ý | Phản biện; Danh mục |
| 4 | Trần 1 USD/ngày không đủ cho video AI | Đúng: RunPod RTX 4090 giá 0,89 USD/giờ; 5 giây 720P mất dưới 9 phút (Đ), nên 1 USD được khoảng 35 giây video (S). Card A5000 24 GB giá 0,27 USD/giờ nhưng em chưa biết tốc độ | Đồng ý | Lời lỗ; Thứ tự xây |
| 5 | Mục danh mục điền sai thì hồ sơ nguồn gốc sai theo | Đúng: repo PhotoCraft ghi MIT hoặc Apache-2.0 và có lệnh `photocraft-cli mcp`, em đã ghi thiếu (Đ) | Đồng ý | Danh mục |
| 6 | Rủi ro mất tài khoản và tệp Suno lớn hơn em mô tả | Đúng: Suno được khóa tài khoản "in its sole discretion" và xóa "all related information and files"; không bảo đảm có bản quyền. Quyền bán của bài đã tải vẫn còn sau khi hủy gói (Đ) | Đồng ý | Danh mục |
| 7 | Gói Suno không nằm trong sổ lời lỗ | Đúng | Đồng ý | Lời lỗ |
| 8 | Nhãn AI phải là dấu máy đọc | Một phần: Luật AI Điều 11.2 giao việc đánh dấu máy đọc cho nhà cung cấp; bên triển khai (công ty) phải thông báo, gắn nhãn theo Điều 11.3 và 11.4 (Đ). Nghị định 142 em chưa đọc | Đồng ý cách sửa (ghi cả siêu dữ liệu vì rẻ), chỉnh lý do | Nguồn gốc |
| 9 | Anh vẫn là nút thắt; chưa tự hành ở đầu ra | Đúng, và đó là chủ ý của giai đoạn đầu | Đồng ý gọi đúng tên và đo phút duyệt | Cổng duyệt |
| 10 | Kiểm link gọi cả địa chỉ nội bộ | Đúng: `content.link_is_live` mở mọi URL, không chặn localhost (Đ đọc code) | Đồng ý | Thay đổi |
| 11 | Ví dụ "gấp 40 lần" dựa trên gói anh đã loại | Đúng | Đồng ý | Lời lỗ |
| 12 | CLI Craft có thể tự biên dịch trên Windows, Linux | Chưa rõ: mã nguồn MIT hoặc Apache-2.0, nhưng README PhotoCraft không ghi cách biên dịch CLI (Đ) | Đồng ý thử trước khi kết luận | Danh mục; Thứ tự xây |
| 13 | Thước đo "tăng trên mỗi USD" chia cho gần 0 | Đúng: công cụ chạy máy gần như không tốn tiền mặt; chi phí thật là token và giờ duyệt của anh | Đồng ý | Loại cược |
| 14 | Dừng sau 10 sản phẩm lỗ liên tiếp thiên về dừng sớm | Đúng: doanh thu một video tích lũy nhiều tháng | Đồng ý | Loại cược |
| 15 | Trần tiền mặt theo từng công ty sẽ nhân lên; trần token danh mục tắt mặc định | Đúng: trần token danh mục chỉ bật khi đặt biến `OPCO_PORTFOLIO_TOKEN_CAP_USD` (Đ đọc code) | Đồng ý | Lời lỗ; Thay đổi |
| 16 | Bộ lỗi mồi quá nhỏ; bộ chữ chưa có ca sạch | Đúng (Đ đọc code) | Đồng ý | Nguồn gốc và kiểm tra |
| 17 | Code còn ghi cứng phần mềm và bản tin ngách; vòng chạy tự mở cược không có trang bán | Đúng: `scout.py` ghi "a software company could test by selling", `content.yaml` ghi "bản tin ngách", `cycle.py` mở cược từ tên ngách (Đ đọc code) | Đồng ý | Thay đổi |
| 18 | Rev 12 không nói nguyên tắc "mỗi lần một hướng" còn giữ không | Đúng | Giữ: chuỗi đầu tiên là một hướng | Chuỗi đầu tiên |

## Nguồn đã mở

Mở ngày 13, 14 và 16/10/2026.

- [ArtCraft, trang giá](https://getartcraft.com/pricing)
- [PhotoCraft](https://getartcraft.com/apps/photocraft), [FilmCraft](https://getartcraft.com/apps/filmcraft), [SoundCraft](https://getartcraft.com/apps/soundcraft)
- [PhotoCraft, repo mã nguồn](https://github.com/storytold/photocraft) (16/10)
- [Điều khoản Suno](https://suno.com/terms)
- [ACE-Step 1.5, trang mô hình](https://huggingface.co/ACE-Step/acestep-v15-base/blob/main/README.md)
- [Kokoro, danh sách giọng](https://huggingface.co/hexgrad/Kokoro-82M/blob/main/VOICES.md) (16/10)
- [BentoML, tổng hợp mô hình giọng nói mở](https://bentoml.com/blog/exploring-the-world-of-open-source-text-to-speech-models) (nguồn thứ cấp, cập nhật 6/4/2026; Kokoro, XTTS-v2)
- [Wan 2.2, README](https://github.com/Wan-Video/Wan2.2) (16/10)
- [Hardwarepedia, Wan 2.2](https://hardwarepedia.com/ai/wan-2-2) (nguồn thứ cấp; số VRAM lệch với README, đã thay)
- [RunPod, trang giá](https://www.runpod.io/pricing) (16/10)
- [YouTube, chính sách kiếm tiền YPP](https://support.google.com/youtube/answer/1311392)
- [YouTube, điều kiện tham gia YPP](https://support.google.com/youtube/answer/72851) (16/10)
- [YouTube Data API, videos.insert](https://developers.google.com/youtube/v3/docs/videos/insert) (16/10)
- [TikTok, Content Sharing Guidelines](https://developers.tiktok.com/doc/content-sharing-guidelines)
- [TikTok, Content Posting API](https://developers.tiktok.com/docs/en/content-posting-api-get-started) (16/10)
- [Dynamoi, chợ nhạc và nhạc AI](https://dynamoi.com/ja/learn/ai-music-distribution/stock-music-sites-that-accept-ai-generated-music) (nguồn thứ cấp, bản phản biện dẫn)
- [Polar, Acceptable Use](https://polar.sh/docs/merchant-of-record/acceptable-use)
- [Luật Thương mại điện tử 122/2025/QH15](https://thuvienphapluat.vn/van-ban/Thuong-mai/Luat-Thuong-mai-dien-tu-2025-so-122-2025-QH15-662035.aspx)
- [Luật Trí tuệ nhân tạo 134/2025/QH15](https://thuvienphapluat.vn/van-ban/Cong-nghe-thong-tin/Luat-Tri-tue-nhan-tao-2025-so-134-2025-QH15-679013.aspx)
- [Phản biện độc lập rev 12](https://claude.ai/artifact/7mVWcHeaiD8ARgwSzbcDqD) (16/10)
