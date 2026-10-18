# Rev 12 (đề xuất): CompanyOS dùng mọi công cụ để làm mọi sản phẩm

Oct 16, 2026 · @Thiệp Nguyễn Quang

**Đề xuất, chưa duyệt.** Rev 11 ở tab chính vẫn là bản đang dùng cho tới khi anh duyệt tab này; khi anh duyệt, em gộp vào tab chính.

CompanyOS thôi gắn cứng vào việc viết bài. Mỗi công cụ (mô hình AI, ứng dụng dòng lệnh, API, MCP) là một mục trong danh mục, ghi rõ nó làm được gì, gọi bằng cách nào, tốn bao nhiêu và có được bán thương mại sản phẩm làm ra không. Mỗi sản phẩm là một công thức ghép các công cụ đó. Mọi tệp làm ra mang theo nguồn gốc, nên code tự gắn nhãn AI, tự chặn vi phạm đã biết trước, và anh chỉ phải duyệt việc lần đầu làm.

## Sửa ngày 16/10 theo phản biện

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

## Em phản biện trước khi thiết kế

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

## Toàn cảnh

&#91;embedded content: vòng từ cơ hội tới tiền · 9 bước, 3 chỗ anh duyệt\]

Vòng chạy theo chiều kim đồng hồ; mỗi vòng là một kỳ phân bổ. Anh chỉ dừng vòng ở ba chỗ, và chỉ khi việc đó làm lần đầu: công cụ mới, cược có thành phần mới, phát hành công khai.

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

## Lời lỗ từng sản phẩm và tiền mặt cho công cụ

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

## Cổng duyệt của anh

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

## Thay đổi so với rev 11

Phần lõi của rev 11 giữ nguyên: hiến chương, nhóm quyết định A–D, sổ cái chỉ thêm, người kiểm khác hãng, bộ phân bổ, nút dừng. Hai biến thể "phần mềm" và "nội dung" thôi là hai nhánh code, trở thành hai bộ công thức và hiến chương mẫu.

| Module | Thay đổi | Mới hay sửa |
| --- | --- | --- |
| Danh mục công cụ | Bảng tools chỉ thêm phiên bản mới; models.yaml gộp vào; mỗi dòng giấy phép kèm câu trích nguyên văn | Mới |
| Bộ chuyển | API, CLI, MCP; điều khiển trình duyệt bật riêng cho công cụ anh duyệt (Suno) | Mới |
| Bộ chọn công cụ | Chọn công cụ rẻ nhất hợp lệ cho một năng lực | Mới |
| Kho tài sản | Bảng assets chỉ thêm, có hồ sơ nguồn gốc; thư mục tệp trên máy; bài Suno tải về ngay sau khi tạo | Mới |
| Hiến chương | Thêm danh sách chặn, loại cược, trần tiền mặt cho công cụ, trần cho cả danh mục, các việc nhóm C mới; mục tiêu thôi gắn với bản tin ngách (charters/content.yaml hiện ghi "bản tin ngách") | Sửa |
| Policy engine | Kiểm thêm: công cụ đã duyệt chưa, quyền đầu ra, điều khoản gọi tự động, danh sách chặn. Trần token danh mục bật mặc định, không chỉ khi đặt biến OPCO\_PORTFOLIO\_TOKEN\_CAP\_USD | Sửa |
| Dây chuyền sản xuất | Từ "mỗi biến thể một bộ luồng" thành "công thức là dữ liệu"; content.py và pipeline.py thành hai công thức | Sửa |
| Bộ kiểm tra và eval | Thêm lớp 1–3 cho media; bộ lỗi mồi theo loại media, mỗi loại ít nhất 20 mẫu lỗi và 20 mẫu sạch. Bộ chữ thêm mẫu sạch và một test chạy người kiểm bằng mô hình thật trên bộ mồi | Sửa |
| Cược | Mỗi loại cược một hàm cổng | Sửa |
| Sổ cái | Ghi chi công cụ (credit, giây, phần gói tháng, kể cả gói Suno); tách tiền mặt với token; ghi số phút anh duyệt | Sửa |
| Kênh | Thành mục danh mục; thêm bộ nối API chính thức | Sửa |
| Bộ điều phối | Hàng đợi theo nơi chạy (máy có card, đám mây). Vòng chạy thôi tự mở cược bán trước chỉ từ tên ngách (cycle.py hiện mở cược với lĩnh vực rỗng, không công thức, không trang bán); mỗi cược phải khai đủ công thức, kênh, cách thu tiền, loại cược và lĩnh vực | Sửa |
| Trinh sát | Thêm trinh sát công cụ; trinh sát cơ hội học từ kết quả thật theo cặp công thức và kênh (Hệ 4); prompt thôi ghi cứng "a software company" (scout.py) | Sửa |
| web.fetch\_text | Kiểm robots.txt trước khi tải (lỗ đã biết) | Sửa |
| Kiểm link (content.link\_is\_live) | Chặn địa chỉ nội bộ và localhost; kiểm robots.txt; tách "bị chặn" khỏi "không tồn tại" | Sửa |

Các khóa hiến chương thêm mới (các khóa khác của rev 11 giữ nguyên; số có dấu `?` là chỗ anh đặt):

```yaml
goal: "Lợi nhuận từ mọi việc pháp luật và đạo đức cho phép"
budget:
  tool_cash_daily_cap_usd: ?             # thuê máy, credit trả theo lần, mỗi công ty
  portfolio_tool_cash_daily_cap_usd: ?   # cộng mọi công ty
  portfolio_token_cap_usd: ?             # bật mặc định, không chờ biến môi trường
bet_types:
  presell:    {days: 14, min_payments: 3}
  audience:   {weeks: ?, measure: [watch_hours, new_subscribers], per: [product, usd, owner_hour]}
  platform:   {cohort_age_days: 28, max_losing_cohorts: ?}
  license:    {days: 30, min_sales_per_asset: ?, exclusive_ai_music: false}
  commission: {days: ?, min_paid_orders: 1}
deny:
  - sell_noncommercial_output
  - automate_tool_that_forbids_bots
  - real_person_voice_or_face_without_consent
  - copyrighted_input_without_license
  - fake_engagement
  - multi_account_evasion
  - fetch_disallowed_by_robots
decision_classes:
  C: [new_tool, new_recipe, new_channel, new_revenue_model, new_market,
      subscription, public_release]   # cộng thêm vào danh sách C của rev 11
```

## Thứ tự xây

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

## Anh cần quyết

Đã quyết ngày 14/10: dùng Suno gói trả phí qua agent điều khiển web; dùng 13 app Craft, không dùng gói ArtCraft. Câu 1 và 2 quyết định giai đoạn A bắt đầu được hay chưa; các câu còn lại chờ được.

| # | Câu hỏi | Em đề xuất |
| --- | --- | --- |
| 1 | Duyệt hướng rev 12, với chuỗi đầu tiên là sản phẩm chữ hoặc PDF tiếng Việt bán qua VietQR? | Duyệt. Em gộp vào tab chính và bắt đầu giai đoạn A |
| 2 | Máy chạy CompanyOS dùng macOS, Windows hay Linux; card đồ họa bao nhiêu GB VRAM? | Quyết định cách gọi app Craft (CLI chỉ có bản macOS) và video AI có chạy trên máy anh được không (cần từ 24 GB) |
| 3 | Agent anh dùng để điều khiển web Suno là agent nào, chạy ở đâu? | CompanyOS gọi agent đó nếu nó nhận lệnh từ dòng lệnh; nếu không, em làm bộ chuyển trình duyệt riêng chạy trên máy anh |
| 4 | Gói Suno của anh giá bao nhiêu mỗi tháng, hạn mức tải bao nhiêu bài? | Cần để ghi vào lời lỗ và đặt trần số bài mỗi ngày |
| 5 | Công cụ tạo ảnh AI và giọng đọc tiếng Việt | Để trinh sát công cụ đề xuất ở giai đoạn B: ưu tiên mô hình mở chạy máy, rồi API trả theo lần. Chuỗi đầu tiên không cần |
| 6 | Phát hành công khai: duyệt từng lần hay theo mẫu; mục tiêu giảm số phút duyệt là bao nhiêu, tới ngày nào? | Từng lần lúc đầu. Đặt mục tiêu sau khi có số phút đo thật của 10 lần duyệt đầu \[S\] |
| 7 | Số cho cổng và trần | Xây khán giả đo sau 8 tuần, theo giờ xem và người đăng ký mới trên mỗi USD và mỗi giờ duyệt; doanh thu nền tảng dừng khi 3 nhóm 28 ngày liên tiếp lỗ; tiền mặt cho công cụ tối đa 1 USD mỗi ngày cho cả danh mục, nằm trong trần 50 USD. Số khởi đầu em chọn, chưa dựa trên số liệu \[S\] |
| 8 | Danh sách chặn 7 mục ở phần Thay đổi | Đồng ý; anh thêm hoặc bỏ mục nào tùy anh |

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
