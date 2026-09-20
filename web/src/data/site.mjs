export const site = Object.freeze({
  name: 'Lumi Agents',
  origin: 'https://agents.runlumi.app',
  repository: 'https://github.com/RunLumi/lumi-agents',
  title: 'Lumi Agents — Giao việc cho AI. Giữ quyền làm chủ.',
  description: 'Lumi Agents đang xây dựng một không gian giao việc cho AI trên tệp, trình duyệt và ứng dụng. Phạm vi rõ ràng, kết quả có thể kiểm tra, con người giữ quyền quyết định.',
  contact: 'mailto:hello@runlumi.app?subject=' + encodeURIComponent('Lumi Agents — Trao đổi về workflow dùng thử') + '&body=' + encodeURIComponent('Chào đội ngũ Lumi,\n\nTôi muốn trao đổi về một workflow đang tốn thời gian:\n- Công việc cần làm:\n- Tần suất:\n- Công cụ đang dùng:\n- Kết quả mong muốn:\n\nThông tin liên hệ của tôi:\n'),
});

export const previewBuild = Boolean(process.env.CF_PAGES_BRANCH && process.env.CF_PAGES_BRANCH !== 'main');

// Marketing examples, not execution evidence. Keep these visibly labelled as demos.
export const workflows = [
  {
    id: 'finance', label: 'Đối soát', glyph: 'evidence', folder: 'doi-soat-thang-09',
    project: 'Tài chính & vận hành', task: 'Đối soát. Tìm chênh lệch. Báo lại.',
    prompt: 'Đối chiếu đơn hàng và giao dịch. Tạo báo cáo chênh lệch, dẫn về từng dòng nguồn. Không sửa dữ liệu gốc.',
    files: ['don-hang.csv', 'giao-dich.csv', 'quy-tac.md'],
    steps: ['Đọc 2 tệp trong thư mục đã chọn', 'Đối chiếu theo mã đơn và số tiền', 'Lưu báo cáo; tách riêng ngoại lệ'],
    result: 'Đã đối chiếu 14 giao dịch mẫu.', summary: '12 khớp. 2 cần bạn xem lại. Dữ liệu nguồn giữ nguyên.',
    artifact: 'doi-soat-mau.csv', artifactLabel: 'Báo cáo đối soát',
    metric: '12 / 14', metricLabel: 'giao dịch khớp trong dữ liệu mẫu',
    rows: [['Mẫu 012', 'Khớp', 'success'], ['Mẫu 013', 'Lệch số tiền', 'attention'], ['Mẫu 014', 'Thiếu mã đơn', 'attention']],
    source: 'don-hang.csv · giao-dich.csv',
    exceptionTitle: 'Vì sao cần bạn xem lại?',
    exception: 'Mẫu 013: giá trị đơn 250.000 ₫, giao dịch 240.000 ₫. Mẫu 014: thiếu mã đơn để đối chiếu. Đây là dữ liệu giả lập; chưa có dữ liệu thật nào được đọc hoặc thay đổi.',
  },
  {
    id: 'research', label: 'Tổng hợp tài liệu', glyph: 'file', folder: 'nghien-cuu-san-pham',
    project: 'Nghiên cứu & tài liệu', task: 'Từ nhiều ghi chú đến một bản rõ ràng.',
    prompt: 'Tổng hợp ghi chú thành một bản brief. Tách điều đã biết, điều chưa rõ và câu hỏi cần xác minh. Gắn tên tệp nguồn cho từng kết luận.',
    files: ['ghi-chu-a.md', 'ghi-chu-b.md', 'yeu-cau.md'],
    steps: ['Đọc 3 tài liệu trong phạm vi dự án', 'Nhóm thông tin; đánh dấu mâu thuẫn', 'Tạo brief kèm nguồn và câu hỏi mở'],
    result: 'Một bản brief để bắt đầu quyết định.', summary: '3 nguồn mẫu. 1 điểm chưa thống nhất. Không biến giả định thành sự thật.',
    artifact: 'brief-mau.md', artifactLabel: 'Brief nghiên cứu',
    metric: '3 nguồn', metricLabel: 'tài liệu minh họa được tham chiếu',
    rows: [['Bối cảnh', 'Có nguồn', 'success'], ['Yêu cầu', 'Có nguồn', 'success'], ['Thời hạn', 'Cần xác minh', 'attention']],
    source: 'ghi-chu-a.md · ghi-chu-b.md · yeu-cau.md',
    exceptionTitle: 'Điều gì chưa được xác minh?',
    exception: 'Hai ghi chú mẫu đưa ra hai thời hạn khác nhau. Bản brief giữ nguyên mâu thuẫn và đặt câu hỏi cho người phụ trách, thay vì tự chọn một thời hạn rồi trình bày như sự thật.',
  },
  {
    id: 'ads', label: 'Kiểm tra quảng cáo', glyph: 'workflow', folder: 'kiem-tra-quang-cao',
    project: 'Marketing & vận hành', task: 'Thấy điểm cần chú ý. Chưa vội thay đổi.',
    prompt: 'Đọc tệp hiệu suất chiến dịch. So sánh với ngưỡng đã đặt, chỉ ra mục cần xem xét và soạn đề xuất. Không thay ngân sách hay tắt quảng cáo.',
    files: ['hieu-suat.csv', 'nguong-kpi.md', 'pham-vi.md'],
    steps: ['Đọc bản xuất dữ liệu đã được cung cấp', 'So sánh 3 chiến dịch mẫu với ngưỡng', 'Soạn đề xuất; chưa áp dụng thay đổi'],
    result: 'Có đề xuất. Quyết định vẫn ở bạn.', summary: '3 chiến dịch mẫu được rà soát. 2 cần chú ý. Không có thay đổi ngân sách.',
    artifact: 'quang-cao-mau.csv', artifactLabel: 'Ghi chú kiểm tra',
    metric: '0 thay đổi', metricLabel: 'ngân sách hoặc trạng thái chiến dịch',
    rows: [['Chiến dịch A', 'Trong ngưỡng', 'success'], ['Chiến dịch B', 'Cần xem lại', 'attention'], ['Chiến dịch C', 'Thiếu dữ liệu', 'attention']],
    source: 'hieu-suat.csv · nguong-kpi.md',
    exceptionTitle: 'Điều gì cần được duyệt?',
    exception: 'Trong quy trình dự kiến, thay đổi ngân sách hoặc trạng thái quảng cáo phải qua chính sách và phê duyệt phù hợp. Bản demo này chỉ đọc dữ liệu giả lập và trình bày đề xuất; không kết nối nền tảng quảng cáo.',
  },
];

export const faqs = [
  ['Lumi Agents khác một chatbot ở đâu?', 'Đích đến là một công việc có đầu ra, không chỉ một câu trả lời. Lumi đang xây dựng luồng chọn phạm vi → giao việc → thực thi có kiểm soát → kiểm tra kết quả. Chat là cách giao tiếp; tệp kết quả, bằng chứng và ngoại lệ mới là phần để bạn đánh giá.'],
  ['Tôi có thể tải về và dùng ngay không?', 'Hiện repo là nền tảng kỹ thuật đang phát triển, chưa phải bản v1 sẵn sàng cho vận hành khách hàng không giám sát. Giao diện trên trang là demo tương tác, không phải agent chạy trực tiếp. Bạn có thể xem mã nguồn hoặc trao đổi với Lumi về một workflow thử nghiệm có giám sát.'],
  ['Dữ liệu có luôn ở trên máy của tôi không?', 'Local-first không đồng nghĩa với offline tuyệt đối. Thiết kế đặt quyền thực thi và phạm vi tệp tại môi trường của người dùng. Khi chọn mô hình hoặc dịch vụ đám mây, nội dung cần thiết có thể được gửi tới nhà cung cấp theo cấu hình và chính sách. Cần kiểm tra đường đi dữ liệu trước khi dùng với dữ liệu nhạy cảm.'],
  ['Lumi có tự gửi email, chi tiền hay xóa dữ liệu không?', 'Theo hợp đồng thiết kế, những tác động quan trọng phải được chính sách cho phép và có phê duyệt khi cần. Tạo bản nháp không đồng nghĩa với gửi; dừng tác vụ không đồng nghĩa với hoàn tác. Các cơ chế này vẫn phải vượt qua kiểm thử thực tế trước khi dùng trong môi trường khách hàng.'],
  ['Lumi hỗ trợ hệ điều hành và mô hình nào?', 'Hướng phát triển là desktop macOS và Windows, với adapter mô hình có thể thay thế. Mức hỗ trợ thực tế phụ thuộc bản dựng, adapter và workflow; đây chưa phải cam kết tương thích sản xuất. Xem tài liệu readiness trong repo để biết điều đã kiểm chứng.'],
  ['Chi phí và cách tham gia dùng thử như thế nào?', 'Runtime trong repo sử dụng giấy phép Apache-2.0. Điều đó không có nghĩa mọi dịch vụ đi kèm đều miễn phí: nhà cung cấp mô hình, hạ tầng và triển khai có thể phát sinh chi phí. Chưa công bố bảng giá thương mại; hãy gửi một workflow cụ thể để trao đổi phạm vi thử nghiệm.'],
];
