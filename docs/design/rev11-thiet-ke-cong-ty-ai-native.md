# Thiết kế công ty AI-native tự hành

Oct 12, 2026 · @Thiệp Nguyễn Quang

## Tóm tắt

Tài liệu này thiết kế **chính công ty**, không chọn sản phẩm cho công ty. Chọn ngách, chọn sản phẩm, định giá, bỏ sản phẩm là việc công ty tự làm khi chạy. Mục tiêu là một mô hình tổ chức và mô hình hoạt động đủ cụ thể để lập trình thành source code, cho hai loại công ty: sản xuất phần mềm và sản xuất nội dung số.

## Cập nhật v2 (13/10): phản biện và thay đổi thiết kế

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

So với v1, thứ tự xây được sửa ở mục Thứ tự xây. Các thay đổi mã nguồn tương ứng chưa làm, chờ bạn duyệt.

**Cách làm.** Đi từ nguyên lý, không từ sơ đồ tổ chức có sẵn. Trước hết xác định những việc mọi doanh nghiệp phải làm để tồn tại và sinh lời. Sau đó hỏi mỗi việc nên do code, agent, dịch vụ ngoài hay con người làm, dựa trên bằng chứng về năng lực agent hiện nay. Phần nghiên cứu và bằng chứng dùng lại từ [báo cáo trước](https://claude.ai/code/artifact/7aef223a-498e-4aa7-adb1-e54b0328ebaa); mã E1–E12 giải thích ở [báo cáo giữa kỳ](https://claude.ai/code/artifact/c56ff2a8-3090-4cee-8d59-d30781b23aa1).

**Kết luận chính.**

1. Công ty tự hành nên được xây như một **hệ thống tự điều chỉnh**: năm lớp chức năng (vận hành, phối hợp, điều khiển, thích nghi, chính sách) theo Mô hình Hệ thống Sống còn (Viable System Model) của Stafford Beer. Đây là lý thuyết về điều kiện để một tổ chức tự duy trì được, không phải một sơ đồ phòng ban.
2. **Agent làm việc, code giữ luật, người giữ chính sách.** Tiền, quyền, sổ sách, ngân sách và điều phối nằm trong code xác định. Agent làm các việc cần hiểu ngôn ngữ và kiểm tra được. Không có CEO agent, vì bằng chứng cho thấy agent yếu nhất ở việc tự điều hành kinh doanh dài hạn (E2, E3).
3. **Công ty tự tìm ngách bằng tiền thật.** Cơ chế trung tâm là danh mục cược: mỗi ý tưởng có ngân sách, bán thử trước khi làm, dừng theo quy tắc. Ngân sách tự chảy về đơn vị có biên đóng góp cao.
4. **Tối ưu chi phí bằng cơ chế, không bằng cắt người.** Định tuyến mô hình rẻ trước, dừng cược nhanh, mua thay vì xây những thứ không tạo khác biệt, và mỗi vai trò agent phải chứng minh giá trị lớn hơn chi phí token cộng chi phí kiểm tra.
5. **Phần con người không bỏ được, nhưng nhỏ.** Một người chịu trách nhiệm pháp lý, đặt chính sách, và duyệt số ít quyết định không đảo ngược được. Luật Việt Nam yêu cầu người đại diện là cá nhân \[Đ\]. Thiết kế nhắm để phần việc này giảm dần khi có số liệu chứng minh an toàn.
6. **Hai loại công ty dùng chung một lõi.** Công ty phần mềm và công ty nội dung khác nhau ở dây chuyền sản xuất và cách kiểm tra chất lượng, giống nhau ở mọi thứ còn lại. Nên lập trình lõi một lần, mỗi loại công ty là một bộ cấu hình và một dây chuyền.

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
| N1 + N3 | **Bán trước khi làm:** mỗi cơ hội phải có người lạ trả tiền trước khi được cấp ngân sách sản xuất | Agent làm trang bán; cổng bằng code | Số thanh toán thật trong 14 ngày | Dưới ngưỡng thì dừng và hoàn tiền tự động |
| N2 Giao giá trị | **Dây chuyền sản xuất có bài nghiệm thu:** việc chia nhỏ, mỗi việc có tiêu chí đạt, người làm khác người kiểm | Agent làm + bộ kiểm tra xác định + agent kiểm khác hãng | Tỷ lệ đạt lần đầu; lỗi lọt tới khách; hoàn tiền | Lỗi lọt tăng thì tăng tỷ lệ lấy mẫu và thêm bài eval; lần làm thứ 3 vẫn hỏng thì leo thang |
| N3 Thu giá trị | **Thu tiền qua bên bán hộ; thử giá trong khung đã duyệt** | Dịch vụ ngoài (Paddle, Polar, VietQR) + agent thử giá | Biên đóng góp mỗi đơn vị; tỷ lệ rời bỏ | Biên âm hai tháng thì đơn vị vào diện xem xét dừng |
| N4 Sống sót | **Ngân sách cứng và đường băng tiền:** trần token theo tháng, theo đơn vị, theo việc; trần cứng ở nhà cung cấp | Code + cấu hình nhà cung cấp | Số tháng còn sống = quỹ / mức đốt ròng | Dưới ngưỡng thì chế độ tiết kiệm: ngừng cược mới, chỉ giữ đơn vị có lãi |
| N5 Phân bổ | **Bộ phân bổ theo quy tắc:** ngân sách kỳ sau = sàn + hệ số × biên đóng góp kỳ trước của từng đơn vị; phần thăm dò cố định cho cược mới | Code; agent chỉ viết báo cáo giải thích | Biên đóng góp trên mỗi USD token, theo đơn vị | Tự động mỗi kỳ; thay đổi quy tắc là nhóm C |
| N6 Nút thắt | **Đo thông lượng từng bước của dây chuyền và của phễu bán**, dồn ngân sách vào bước nghẽn | Code đo; agent phân tích | Thời gian chờ, tỷ lệ chuyển đổi từng bước | Bước nghẽn được ưu tiên trong hàng đợi; không tăng sản xuất khi phân phối nghẽn |
| N7 Phối hợp | **Bộ điều phối xác định:** hàng đợi việc, luồng viết sẵn, khóa tài nguyên, đầu ra có lược đồ; agent không nhắn tin tự do với nhau | Code (workflow engine) | Việc kẹt, vòng lặp, xung đột tài nguyên | Ngắt vòng lặp theo số lượt; việc kẹt quá hạn thì hủy hoặc leo thang |
| N8 Kiểm soát | **Tách quyền trong code:** mỗi vai trò một khóa, quyền tối thiểu; sổ cái chỉ ghi từ dữ liệu nguồn; bảng quyền A–D; **kiểm toán độc lập** bằng lấy mẫu ngẫu nhiên và lỗi mồi | Code + agent kiểm toán khác hãng + bạn xem mẫu | Tỷ lệ bắt lỗi mồi; chênh lệch đối soát | Bắt lỗi mồi dưới ngưỡng thì hạ quyền loại việc đó về nhóm C |
| N9 Thích nghi | **Quét môi trường và học từ kết quả:** theo dõi đối thủ, giá mô hình, chính sách nền tảng, luật mới; mỗi cược dừng để lại bài học; mỗi lỗi thành bài eval | Agent quét + bộ nhớ tổ chức | Số thay đổi môi trường ảnh hưởng đơn vị đang chạy | Đề xuất sửa chính sách gửi bạn (học hai vòng); sửa quy trình trong khung thì tự làm qua eval |
| N10 Chính sách | **Hiến chương công ty:** tệp cấu hình có phiên bản ghi mục tiêu, giới hạn chi, danh mục cấm, bảng quyền, ngưỡng | Bạn soạn và duyệt; code thực thi | Số lần hành động bị chặn bởi hiến chương | Chỉ bạn sửa; agent được đề xuất sửa kèm số liệu |
| N11 Trách nhiệm | **Người chịu trách nhiệm có tên, lời hứa chỉ từ chính sách đã duyệt, nhật ký không sửa được** | Bạn; code ghi nhật ký | Khiếu nại, tranh chấp | Khiếu nại pháp lý luôn lên bạn; giữ lời hứa đã đưa ra rồi sửa nguồn lỗi |
| N12 Ranh giới | **Quy tắc tự làm hay mua:** mua thứ có trách nhiệm pháp lý hoặc không tạo khác biệt (thanh toán, thuế, hosting); giữ trong nhà dữ liệu, bộ eval, prompt, quy trình | Quy tắc trong hiến chương | Chi phí dịch vụ ngoài so với chi phí tự làm | Nhà cung cấp mới là nhóm C |

**Ba cơ chế này là xương sống** \[S\]: bán trước khi làm (N1, N3), bộ phân bổ theo quy tắc (N5), và kiểm toán độc lập bằng lỗi mồi (N8). Thiếu cơ chế thứ nhất, công ty tự hành sẽ sản xuất rất hiệu quả những thứ không ai mua. Thiếu cơ chế thứ hai, nó không tự rút khỏi chỗ lỗ. Thiếu cơ chế thứ ba, không ai biết chất lượng thật đang ở đâu khi bạn không nhìn.

## Mô hình tổ chức

Tổ chức được chia theo chức năng sống còn, không theo phòng ban. Mỗi hệ ghi rõ ai đảm nhận: người, agent, code hay dịch vụ ngoài.

&#91;embedded content: mô hình tổ chức · 5 hệ theo VSM, nền tảng, ngoài biên\]

Lệnh và luật đi từ trên xuống. Thông tin đi lên chỉ dưới dạng ngoại lệ đã lọc: hệ 3 gom chỉ số của mọi đơn vị và chỉ đưa lên bạn những gì vượt ngưỡng. Hệ 3\* đi thẳng xuống hệ 1 để lấy mẫu, không qua báo cáo của chính đơn vị, vì đơn vị tự báo cáo về mình là không độc lập (E5).

### Vai trò agent

Mỗi vai trò là một bản mô tả có phiên bản: đầu vào, đầu ra theo lược đồ, công cụ được dùng, quyền, ngân sách mỗi việc, bài nghiệm thu, mô hình mặc định, và khi nào leo thang. Một vai trò chỉ được tạo khi chứng minh được giá trị lớn hơn chi phí token cộng chi phí kiểm tra của nó.

| Vai trò | Hệ | Làm gì | Không được làm | Ai kiểm |
| --- | --- | --- | --- | --- |
| Trinh sát thị trường | 4 | Đọc dữ liệu công khai, viết thẻ cơ hội | Đăng bài, chi tiền, xem dữ liệu khách | Thang chấm bằng code |
| Trinh sát môi trường | 4 | Theo dõi luật, chính sách nền tảng, giá mô hình, đối thủ | Thay đổi cấu hình | Bạn đọc tóm tắt khi có thay đổi lớn |
| Người bán thử | 1 | Làm trang bán, ưu đãi, bài giới thiệu cho cược | Hứa ngoài chính sách; đánh giá giả | Cổng kiểm tra nội dung + luật đạo đức |
| Người làm (phần mềm hoặc nội dung) | 1 | Thực hiện việc nhỏ có bài nghiệm thu | Giữ khóa production; tự duyệt việc của mình | Người kiểm |
| Người kiểm | 1 | Chạy test, đối chiếu nguồn, chấm theo tiêu chí; dùng mô hình khác hãng với người làm | Sửa việc mình kiểm | Kiểm toán (lỗi mồi) |
| Người phát hành | 1 | Triển khai hoặc đăng bài có tự hoàn tác | Xóa dữ liệu; migration không qua hai khóa | Giám sát sau phát hành |
| Người phân phối | 1 | Soạn và đăng bài trên kênh đã duyệt | Mở kênh mới; giả làm người thật | Cổng kiểm tra + giới hạn tốc độ |
| Hỗ trợ khách | 1 | Trả lời từ chính sách đã duyệt, tự giới thiệu là AI, hoàn tiền dưới trần | Hứa ngoài chính sách | Lấy mẫu hội thoại |
| Kế toán viên (chỉ đọc) | 3 | Đối soát sổ cái với dữ liệu bên bán hộ, ngân hàng | Chuyển tiền; sửa sổ | Đối soát hai chiều bằng code |
| Phân tích danh mục | 3 | Viết giải thích cho quyết định của bộ phân bổ; đề xuất | Tự đổi ngân sách | Bạn khi đề xuất thuộc nhóm C |
| Kiểm toán viên | 3\* | Lấy mẫu ngẫu nhiên, cài lỗi mồi, đo tỷ lệ bắt lỗi | Sửa việc được kiểm | Bạn xem báo cáo kiểm toán |
| Người soạn hồ sơ | 5 | Chuẩn bị hồ sơ cho việc nhóm C: tóm tắt, bằng chứng, lựa chọn | Quyết định | Bạn |

Không có vai trò CEO, quản lý hay trưởng phòng. Những gì một quản lý làm được tách ra: phân bổ (code), điều phối (code), kiểm soát chất lượng (người kiểm và kiểm toán), quyết định khó đảo ngược (bạn).

### Con người

- **Chủ sở hữu (bạn):** hệ 5. Viết và sửa hiến chương; duyệt nhóm C; ký mọi thứ thuộc nhóm D; xử lý sự cố mức 3. Không đọc nhật ký hằng ngày, chỉ đọc bảng ngoại lệ và báo cáo kiểm toán.
- **Chuyên gia thuê theo việc:** kế toán, luật sư. Công ty tự soạn hồ sơ, người chuyên môn kiểm và ký.
- **Người duyệt mẫu (khi có điều kiện):** thuê khi tỷ lệ bắt lỗi mồi dưới ngưỡng, hoặc khi giờ của bạn vượt ngưỡng. Trả từ doanh thu.

### Ranh giới quyền

Bảng quyền A–D dùng lại từ báo cáo trước: A tự làm; B tự làm, có kiểm mẫu; C bạn duyệt; D không bao giờ giao agent. Một loại việc chuyển từ C xuống B khi có ba điều kiện: cách hoàn tác đã thử, trần thiệt hại trong code, và bài eval cho loại việc đó đã bắt được lỗi mồi. Đây là cơ chế để phần việc của con người giảm dần theo số liệu, không theo cảm giác.

## Mô hình hoạt động

Công ty chạy bằng năm vòng lặp lồng nhau, từ nhanh đến chậm. Vòng nhanh do agent và code tự xử lý; càng lên vòng chậm, quyết định càng ít và càng quan trọng. Vòng chậm đặt luật cho vòng nhanh; vòng nhanh gửi số liệu lên vòng chậm. Không có họp định kỳ; nhịp do sự kiện và bộ hẹn giờ quyết định.

| Vòng | Nhịp | Việc | Ai quyết | Đầu ra |
| --- | --- | --- | --- | --- |
| V1 Việc | Phút đến giờ | Nhận việc → làm → kiểm → đạt thì chấp nhận; không đạt thì làm lại tối đa 2 lần rồi leo thang | Code (bài nghiệm thu) | Sản phẩm đã kiểm; chi phí ghi sổ |
| V2 Đơn vị | Theo sự kiện, hằng ngày | Mỗi đơn vị xử lý đơn hàng, ticket, bản phát hành, bài đăng; tự xếp việc theo ưu tiên của bước nghẽn | Code + agent trong quyền A, B | Số liệu đơn vị: doanh thu, chi phí, lỗi, khiếu nại |
| V3 Danh mục | Hằng tuần | Bộ phân bổ tính lại ngân sách; cược nào đến cổng thì qua hoặc dừng; mở cược mới từ thẻ cơ hội điểm cao nhất | Code theo quy tắc; vượt mặc định là nhóm C | Bảng ngoại lệ tuần cho bạn |
| V4 Thích nghi | Hằng tháng | Hiệu chỉnh thang chấm ngách theo kết quả thật; cập nhật bộ eval; xem lại mô hình nào rẻ hơn mà vẫn đạt eval; báo cáo kiểm toán | Agent đề xuất; đổi quy trình trong khung thì tự làm qua eval | Đề xuất sửa chính sách kèm số liệu |
| V5 Chính sách | Khi cần, tối thiểu hằng quý | Sửa hiến chương: mục tiêu, trần chi, danh mục cấm, chuyển việc giữa nhóm C và B | Bạn | Hiến chương phiên bản mới |

### Sự kiện kích hoạt

Công việc sinh ra từ sự kiện, không từ kế hoạch. Các sự kiện chính: có thanh toán mới; có yêu cầu hoàn tiền; có ticket hỗ trợ; test thất bại; bản phát hành lỗi; một cược đến cổng; chi vượt ngưỡng; trinh sát phát hiện thay đổi môi trường; kiểm toán phát hiện lỗi lọt; bạn sửa hiến chương. Danh sách sự kiện đầy đủ nằm ở phần Đặc tả để lập trình.

### Leo thang

1. **Mức 1, tự xử lý:** bài nghiệm thu chặn việc hỏng và trả lại người làm.
2. **Mức 2, vào bảng ngoại lệ:** vượt ngưỡng chi phí, tỷ lệ lỗi, khiếu nại, hoặc việc hỏng sau 3 lần. Bạn xem theo tuần.
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
| Số việc nhóm C chờ bạn | ≤ 5 | Giảm số cược song song cho đến khi hàng đợi vơi |
| Thời gian chờ ở một bước dây chuyền | ≤ 2 lần trung vị | Ưu tiên bước đó trong hàng đợi; không nhận việc mới vào đầu dây chuyền |
| Số lượt gọi mô hình mỗi việc | ≤ trần của vai trò | Ngắt vòng lặp; việc vào mức 2 |

### Công ty tự cải tiến thế nào

Agent được đề xuất sửa cách công ty làm việc: prompt, quy trình, chọn mô hình, ngưỡng. Mọi thay đổi phải chạy qua bộ eval trước, so với phiên bản cũ, rồi áp dụng cho một phần nhỏ việc trước khi áp dụng toàn bộ. Thay đổi nằm trong khung của hiến chương thì tự động; thay đổi hiến chương thì lên bạn. Agent không bao giờ được sửa bộ eval dùng để chấm chính nó, vì đó là cách nhanh nhất để “đạt” mà không tốt lên \[S\].

## Kinh tế của công ty

Phần này không dự báo doanh thu. Nó mô tả công thức lợi nhuận của công ty, các cần gạt tối ưu chi phí, và các tham số mà code sẽ điều chỉnh. Số cụ thể sẽ là số liệu vận hành thật của công ty.

### Công thức lợi nhuận

Lợi nhuận kỳ = Σ biên đóng góp của các đơn vị − chi phí thăm dò − chi phí nền tảng − chi phí chuyên gia.

- **Biên đóng góp của một đơn vị** = doanh thu − phí thanh toán − token sản xuất và kiểm tra − token hỗ trợ − hạ tầng riêng − hoàn tiền.
- **Chi phí thăm dò** = token cho thẻ cơ hội và bán thử của các cược chưa thành đơn vị. Đây là “chi phí R&D” của công ty tự hành.
- **Chi phí nền tảng** = hạ tầng chung, eval, kiểm toán, trinh sát môi trường. Gần như cố định.
- **Giờ của bạn** không phải tiền mặt nhưng là chi phí kinh tế; một công ty “có lãi” nhưng cần 20 giờ/tuần của bạn thì chưa tự hành. Nó được đo và báo cáo cùng lợi nhuận.

### Quy tắc ngân sách tự nuôi

Ngân sách token kỳ sau = B0 + α × biên đóng góp trung bình 3 kỳ gần nhất.

- B0 là sàn chủ sở hữu cấp. Bạn đã chọn khoảng 100 USD/tháng cho giai đoạn thử nghiệm.
- α (0,3–0,5 \[GT\]) là phần lợi nhuận được tái đầu tư; phần còn lại là lợi nhuận giữ lại hoặc quỹ dự phòng.
- Ngân sách chia hai phần: **khai thác** (cho đơn vị đang có lãi, theo tỷ lệ biên đóng góp trên mỗi USD token) và **thăm dò** (tỷ lệ cố định, ví dụ 30% \[GT\], cho cược mới). Đây là bài toán khai thác và thăm dò kinh điển; giữ phần thăm dò cố định để công ty không chết cùng một sản phẩm đang suy \[LT\].
- Tiền mặt ngoài token chỉ chi từ doanh thu đã về, trừ khoản trần khoảng 50 USD trước doanh thu bạn đã duyệt.

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
| Tỷ lệ can thiệp của người | Giờ của bạn và số việc nhóm C trên mỗi triệu doanh thu | Đo mức tự hành thật; cũng là số liệu chính khi bán mô hình |
| Tỷ lệ lỗi lọt tới khách | Lỗi khách phát hiện trên tổng giao hàng | Chất lượng thật |
| Tỷ lệ bắt lỗi mồi | Người kiểm bắt được bao nhiêu lỗi cài sẵn | Chất lượng của bước kiểm tra |
| Số tháng còn sống | Quỹ / mức đốt ròng | Sống sót |

Với mục tiêu sau này gọi vốn hoặc bán mô hình công ty số, ba chỉ số có giá trị nhất là: tỷ lệ can thiệp của người, chi phí tìm được một đơn vị có lãi, và biên đóng góp trên mỗi USD token \[S\]. Cả ba đều phải được ghi từ ngày đầu, vì nhà đầu tư tin số liệu lịch sử hơn số liệu tự báo cáo (E10).

## Hai biến thể: công ty phần mềm và công ty nội dung số

Hai công ty dùng chung toàn bộ hệ 2, 3, 3\*, 4, 5 và nền tảng. Chúng khác nhau ở hệ 1: dây chuyền sản xuất, cách kiểm tra chất lượng, cách kiếm tiền và rủi ro riêng.

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

**Nên lập trình biến thể nào trước?** Đề xuất: công ty phần mềm \[S\]. Lý do chính là kiểm tra chất lượng: test là bộ kiểm tra xác định, rẻ và đáng tin, nên cơ chế kiểm toán bằng lỗi mồi hoạt động tốt nhất và vai trò của bạn giảm nhanh nhất. Thêm nữa, bạn là lập trình viên và có cộng đồng lập trình, nên có thể tự kiểm tra bộ kiểm tra trong giai đoạn đầu. Công ty nội dung là biến thể thứ hai, chỉ cần viết dây chuyền và bộ kiểm tra mới trên cùng một lõi. Đây là lựa chọn bạn có thể đổi; nó không ảnh hưởng đến thiết kế lõi.

## Đặc tả để lập trình

Phần này là cầu nối sang source code. Nó nói cần những module nào, dữ liệu nào, sự kiện nào, cấu hình nào, và xây theo thứ tự nào. Chưa chọn ngôn ngữ hay framework; đó là câu hỏi mở ở cuối tài liệu.

### Module

| Module | Hệ | Trách nhiệm | Agent hay code |
| --- | --- | --- | --- |
| Hiến chương | 5 | Đọc, kiểm tra hợp lệ, lưu phiên bản tệp cấu hình công ty; chỉ bạn ghi được | Code |
| Policy engine | 5, 3 | Trước mọi hành động: xác định nhóm A–D, kiểm trần, cho phép hoặc chặn hoặc đưa vào hàng duyệt | Code |
| Hàng duyệt | 5 | Việc nhóm C kèm hồ sơ; bạn duyệt hoặc từ chối; có hạn chờ | Code + agent soạn hồ sơ |
| Sổ cái | Nền tảng | Ghi mọi khoản thu, chi, token theo đơn vị và vai trò; chỉ thêm, không sửa | Code |
| Bộ phân bổ | 3 | Tính ngân sách mỗi kỳ theo quy tắc; mở, dừng cược theo cổng | Code |
| Bộ điều phối | 2 | Hàng đợi việc, chạy luồng, khóa tài nguyên, thử lại, ngắt vòng lặp | Code |
| Agent runtime | 1, 4 | Chạy một vai trò: nạp mô tả vai trò, chọn mô hình, cấp công cụ và khóa theo quyền, đếm token | Code bao quanh agent |
| Bộ kiểm tra | 1 | Chạy bài nghiệm thu xác định, rồi người kiểm bằng LLM khác hãng | Code + agent |
| Eval harness | 3\*, 4 | Bộ bài kiểm tra có lỗi mồi; chạy khi đổi mô hình, prompt, quy trình; lấy mẫu ngẫu nhiên | Code + agent kiểm toán |
| Bộ nhớ tổ chức | Nền tảng | Quyết định, chính sách đã duyệt, bài học; có hạn dùng và người duyệt | Code |
| Danh tính và khóa | Nền tảng | Mỗi vai trò, mỗi đơn vị một khóa có trần chi; thu hồi khóa = nút dừng | Code |
| Nhật ký và bảng ngoại lệ | 3 | Ghi mọi hành động; tính chỉ số; phát cảnh báo theo ngưỡng | Code + agent tóm tắt |
| Cổng kết nối | Nền tảng | Bên bán hộ, ngân hàng, git, hosting, CMS, email, nhà cung cấp mô hình | Code |
| Dây chuyền sản xuất | 1 | Mỗi biến thể một bộ luồng và bộ kiểm tra riêng | Cấu hình + agent |

### Thực thể dữ liệu chính

- **Charter** (hiến chương): phiên bản, mục tiêu, trần, danh mục cấm, bảng quyền, ngưỡng.
- **Opportunity** (thẻ cơ hội): vấn đề, người trả tiền, tín hiệu, điểm, loại rủi ro.
- **Bet** (cược): giả thuyết, ngân sách, cổng, hạn, trạng thái.
- **Unit** (đơn vị): cấu hình, dây chuyền, sổ lãi lỗ, khóa.
- **RoleSpec** (mô tả vai trò): đầu vào, đầu ra, công cụ, quyền, ngân sách, mô hình, bài nghiệm thu.
- **Task** (việc), **Artifact** (sản phẩm của việc), **Verdict** (kết quả kiểm tra).
- **Decision** (quyết định nhóm C: hồ sơ, người duyệt, kết quả).
- **LedgerEntry** (bút toán), **Metric** (chỉ số), **Incident** (sự cố), **MemoryItem** (mục bộ nhớ).
- **Customer**: chỉ lưu tối thiểu để giao hàng và hỗ trợ; dữ liệu cá nhân không được gửi vào mô hình.

### Sự kiện

`opportunity.scored` · `bet.opened` · `bet.gate_reached` · `bet.killed` · `unit.created` · `task.created` · `task.completed` · `verify.failed` · `verify.passed` · `release.deployed` · `release.rolled_back` · `payment.received` · `refund.requested` · `ticket.opened` · `budget.threshold` · `runway.low` · `audit.seeded_error_missed` · `environment.changed` · `approval.requested` · `approval.decided` · `incident.opened` · `charter.updated`

### Hiến chương mẫu

Ví dụ hình dạng tệp cấu hình, số là giá trị khởi điểm \[GT\]:

```yaml
charter_version: 1
owner: thiep
goal: "Lợi nhuận từ sản phẩm số, hợp pháp, không lừa dối"
budget:
  base_monthly_usd: 100      # B0
  reinvest_ratio: 0.4        # alpha
  explore_share: 0.3
  daily_cap_ratio: 0.05
  cash_before_revenue_usd: 50
gates:
  presell_days: 14
  presell_min_payments: 3
  unit_min_positive_months: 2
  pivots_allowed: 1
forbidden_domains: [high_risk_ai, health, personal_finance, personal_legal, children, sensitive_data]
decision_classes:
  C: [new_vendor, first_launch, price_model_change, prod_migration, new_personal_data, legal_reply, charter_change]
  D: [sign_contract, file_tax, move_money_out, open_account, impersonate_human, fake_review]
thresholds:
  seeded_error_catch_min: 0.8
  refund_rate_max: 0.10
  approval_queue_max: 5
  runway_months_min: 3
```

### Thứ tự xây

1. **Lõi không có agent:** hiến chương, policy engine, sổ cái, khóa và nút dừng, nhật ký, hàng duyệt. Kiểm bằng: chạy 10 kịch bản kiểm thử sức chịu đựng ở báo cáo trước với hành động giả lập.
2. **Một dây chuyền đầu cuối:** bộ điều phối, agent runtime, bộ kiểm tra, eval harness có lỗi mồi, cho biến thể phần mềm. Kiểm bằng: tỷ lệ bắt lỗi mồi và chi phí token mỗi việc.
3. **Vòng cược:** trinh sát thị trường, thẻ cơ hội, bán thử, cổng, kết nối bên bán hộ. Từ đây công ty bắt đầu tự tìm ngách.
4. **Bộ phân bổ và bảng ngoại lệ:** khi có ít nhất một đơn vị có doanh thu.
5. **Biến thể nội dung số:** dây chuyền và bộ kiểm tra mới trên cùng lõi.

Bước 1 cố ý không có agent. Lý do: mọi sự cố nặng trong nghiên cứu đều xảy ra khi agent có quyền trước khi có hàng rào (E7). Xây hàng rào trước, rồi mới cho agent vào.

**Cập nhật v2, thứ tự mới:** (0) sửa lỗi đã biết và đặt trần chi trước khi đặt khóa API thật; (1) vòng chạy tối thiểu nối trinh sát, cược và cổng, có trần lượt gọi; (2) thước đo đúng: ca sạch trong eval, chi token theo đơn vị, phê duyệt dùng một lần; (3) biến thể nội dung với hiến chương và bộ mồi nội dung, bán một bản nhỏ trước; (4) vai trò phân phối trên một kênh đã duyệt; (5) bộ phân bổ và bảng ngoại lệ, khi có đơn vị có doanh thu; (6) biến thể phần mềm.

## Rủi ro của mô hình và câu hỏi cần bạn quyết

### Rủi ro của chính mô hình

- **Công ty tự tìm ngách là giả định rủi ro nhất.** Chưa có bằng chứng công khai về một công ty agent tự tìm được ngách có lãi bền vững mà không có người hiểu ngách đứng sau \[S\]. Thiết kế này không giả định agent đoán giỏi; nó dùng tiền thật làm trọng tài. Nếu phân phối không tốn tiền không mang được khách tới, cơ chế vẫn đúng nhưng sẽ chỉ báo “dừng” liên tục.
- **Người kiểm cùng mắc lỗi với người làm.** Lỗi giữa các mô hình có tương quan (E3). Lỗi mồi đo được lỗi đã biết, không đo được lỗi chưa ai nghĩ ra.
- **Luật chơi sai thì tự hành sai một cách đều đặn.** Ngưỡng trong hiến chương ban đầu là đoán; công ty sẽ tối ưu theo chúng cho đến khi bạn sửa.
- **Bạn là điểm hỏng duy nhất của hệ 5.** Khi bạn vắng, công ty chỉ giữ nguyên trạng (chế độ an toàn), không phát triển.
- **Tiêm prompt và quyền quá rộng** vẫn là rủi ro bảo mật lớn nhất (E7, E9). Kiểm soát bằng kiến trúc quyền, không bằng prompt.
- **Rủi ro pháp lý** đã liệt kê ở sổ nghĩa vụ của báo cáo trước vẫn giữ nguyên; quan trọng nhất là chuyển dữ liệu cá nhân xuyên biên giới và nhãn nội dung AI.

### Điều gì chứng minh mô hình sai

| Nếu thấy | Thì |
| --- | --- |
| Tỷ lệ cược thành đơn vị bằng 0 sau khoảng 10 cược | Cơ chế tìm ngách cần thêm nguồn lợi thế: kênh riêng, người hiểu ngách, hoặc ngân sách quảng cáo nhỏ |
| Tỷ lệ can thiệp của người không giảm sau 3 tháng | Thiết kế quyền quá bảo thủ, hoặc agent chưa đủ năng lực cho loại việc đó; xem từng loại việc |
| Biên đóng góp trên mỗi USD token không tăng | Agent đang làm việc tốn hơn giá trị; bỏ bước hoặc đổi mô hình |
| Nghiên cứu mới cho thấy agent tự điều hành kinh doanh dài hạn tốt | Cho agent đảm nhận thêm phần hệ 3 và hệ 4; hệ 5 vẫn là người vì luật |

### Câu hỏi cần bạn quyết trước khi lập trình

1. **Biến thể đầu tiên:** phần mềm (đề xuất) hay nội dung số?
2. **Công nghệ:** ngôn ngữ, framework, nơi chạy (VPS, serverless, máy của bạn), nhà cung cấp mô hình. Bạn có ưu tiên nào không?
3. **Mức tự hành lúc khởi đầu:** bảng nhóm C trong hiến chương mẫu có quá nhiều hay quá ít với bạn?
4. **Công ty có được tự thuê người (cộng tác viên) bằng doanh thu không,** hay việc thuê luôn là nhóm C?
5. **Một công ty hay nhiều công ty:** mã nguồn này chạy một công ty của bạn, hay được thiết kế ngay từ đầu để chạy nhiều công ty (để sau này bán mô hình)? Đề xuất: một công ty trước, nhưng giữ hiến chương và dây chuyền tách khỏi code lõi để nhân bản sau.

   **Câu hỏi mới (v2): thu tiền nội dung ở Việt Nam bằng cách nào?** Đề xuất: chuyển khoản VietQR vào tài khoản cá nhân và tự kê khai, sau khi kế toán xác nhận ngưỡng thuế hiện hành. Hai lựa chọn khác: đăng ký hộ kinh doanh ngay, hoặc chỉ bán quốc tế qua Polar (vướng chính sách đặt trước và eBook, trái với “Việt Nam trước”).
