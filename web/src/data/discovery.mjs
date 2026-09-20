import { site, faqs } from './site.mjs';

// RunLumi publishes Lumi Agents. Do not rename the organization to the product
// merely to satisfy a third-party brand-consistency heuristic.
export const publisher = Object.freeze({
  name: 'RunLumi',
  url: 'https://runlumi.app',
  id: 'https://runlumi.app/#organization',
  logo: `${site.origin}/brand/lumi-logo.svg`,
  sameAs: ['https://github.com/RunLumi'],
});

const docs = `${site.repository}/blob/main/docs`;
export const productFacts = Object.freeze([
  {
    label: 'Trạng thái',
    value: 'Đang phát triển, chưa phát hành v1. Chưa sẵn sàng cho vận hành khách hàng không giám sát; có thể trao đổi về một workflow thử nghiệm có giám sát.',
    source: `${docs}/v1-readiness.md`, sourceLabel: 'Trạng thái kiểm chứng',
  },
  {
    label: 'Phạm vi sản phẩm',
    value: 'Hướng tới giao việc cho AI trên tệp, trình duyệt và ứng dụng, với đầu ra, nguồn tham chiếu và ngoại lệ để con người kiểm tra. Các ví dụ trên trang là dữ liệu giả lập.',
    source: `${docs}/implementation-truth.md`, sourceLabel: 'Điều đã có và chưa có',
  },
  {
    label: 'Nền tảng',
    value: 'Hướng phát triển là desktop macOS và Windows, với adapter mô hình có thể thay thế. Khả năng thực tế phụ thuộc bản dựng, adapter và workflow, không phải cam kết tương thích sản xuất.',
    source: `${docs}/roadmap.md`, sourceLabel: 'Hướng phát triển',
  },
  {
    label: 'Dữ liệu và quyền hạn',
    value: 'Local-first không đồng nghĩa offline tuyệt đối. Dịch vụ đám mây có thể nhận nội dung theo cấu hình. Gửi ra ngoài, chi tiền, xóa dữ liệu và đổi quyền phải qua chính sách, phê duyệt và kiểm chứng phù hợp.',
    source: `${site.repository}/blob/main/SECURITY.md`, sourceLabel: 'Chính sách bảo mật',
  },
  {
    label: 'Giấy phép và chi phí',
    value: 'Mã nguồn runtime sử dụng Apache-2.0. Chưa công bố bảng giá thương mại; mô hình, hạ tầng và triển khai có thể phát sinh chi phí.',
    source: `${site.repository}/blob/main/LICENSE`, sourceLabel: 'Giấy phép mã nguồn',
  },
]);

export function createStructuredData({ pathname = '/', title = site.title, description = site.description, includeFaq = false } = {}) {
  const canonical = new URL(pathname, site.origin).href;
  const websiteId = `${site.origin}/#website`;
  const pageId = `${canonical}#webpage`;
  const faqId = `${site.origin}/#cau-hoi`;
  // FAQ data describes the visible homepage section, never a 404 or other page.
  const hasFaq = includeFaq && pathname === '/';
  const graph = [
    {
      '@type': 'Organization', '@id': publisher.id,
      name: publisher.name, url: publisher.url,
      logo: publisher.logo, sameAs: publisher.sameAs,
    },
    {
      '@type': 'WebSite', '@id': websiteId,
      name: site.name, url: `${site.origin}/`,
      description: site.description, inLanguage: 'vi-VN',
      publisher: { '@id': publisher.id },
    },
    {
      '@type': 'WebPage', '@id': pageId,
      name: title, url: canonical, description, inLanguage: 'vi-VN',
      isPartOf: { '@id': websiteId },
      author: { '@id': publisher.id }, publisher: { '@id': publisher.id },
      ...(hasFaq ? { hasPart: { '@id': faqId } } : {}),
    },
  ];
  if (hasFaq) graph.push({
    '@type': 'FAQPage', '@id': faqId, url: faqId,
    name: 'Lumi Agents — Câu hỏi thường gặp', inLanguage: 'vi-VN',
    isPartOf: { '@id': pageId },
    author: { '@id': publisher.id }, publisher: { '@id': publisher.id },
    mainEntity: faqs.map(([question, answer]) => ({
      '@type': 'Question', name: question,
      acceptedAnswer: { '@type': 'Answer', text: answer },
    })),
  });
  return { '@context': 'https://schema.org', '@graph': graph };
}

// JSON-LD is serialized once, with script-breaking characters escaped. The
// postbuild script hashes the emitted bytes into the existing strict CSP.
export function serializeStructuredData(value) {
  return JSON.stringify(value)
    .replace(/</g, '\\u003c')
    .replace(/\u2028/g, '\\u2028')
    .replace(/\u2029/g, '\\u2029');
}

export function renderLlmsText({ preview = false } = {}) {
  if (preview) return `# ${site.name} — bản xem trước\n\n> Đây không phải website chính thức để lập chỉ mục.\n\n## Website chính thức\n\n- [${site.name}](${site.origin}/): Nội dung sản phẩm được công bố.\n`;
  return [
    `# ${site.name}`, '', `> ${site.description}`, '',
    `Website chính thức: ${site.origin}/. Biên soạn và duy trì bởi ${publisher.name} (${publisher.url}/).`,
    'Trang giới thiệu và demo không phải bằng chứng agent đã thực thi hoặc sản phẩm đã sẵn sàng cho vận hành không giám sát.',
    '', '## Thông tin sản phẩm', '',
    ...productFacts.map(fact => `- [${fact.label}](${fact.source}): ${fact.value}`),
    '', '## Nội dung chính thức', '',
    `- [Giới thiệu Lumi Agents](${site.origin}/): Sản phẩm, ví dụ minh họa và giới hạn hiện tại.`,
    `- [Thông tin trước khi dùng thử](${site.origin}/#thong-tin): Tóm tắt phạm vi, trạng thái, nền tảng, dữ liệu và chi phí.`,
    `- [Câu hỏi thường gặp](${site.origin}/#cau-hoi): Câu trả lời đầy đủ về sử dụng, dữ liệu, quyền hạn và chi phí.`,
    `- [Trao đổi workflow dùng thử](${site.origin}/#bat-dau): Mở email tới hello@runlumi.app; không đăng ký hoặc gửi tự động.`,
    `- [Mã nguồn](${site.repository}): Mã nguồn công khai và lịch sử thay đổi.`,
    '',
  ].join('\n');
}
