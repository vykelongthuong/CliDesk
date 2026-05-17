# CliDesk - Prompt mẫu cho AI Coding Agent

File này là hướng dẫn bắt buộc phải đọc trước khi AI coding agent sửa dự án CliDesk.

Mỗi lần nhận task, agent phải đọc file này và tuân thủ. Nếu task có yêu cầu riêng từ user thì yêu cầu riêng được ưu tiên, nhưng không được vi phạm các quy tắc an toàn trong file này.

## 1. Thông tin dự án

- Tên dự án: CliDesk
- Stack: Tauri v2 + React + TypeScript + Rust
- Mục tiêu: desktop app local để quản lý nhiều terminal/CLI coding agent theo project.
- Root local thường dùng: `E:\code\CliDesk`
- Repo GitHub: `https://github.com/vykelongthuong/CliDesk`

## 2. Quy tắc làm việc bắt buộc

- Luôn đọc source hiện tại trước khi sửa.
- Không phỏng đoán nếu có thể kiểm tra bằng source.
- Không rewrite toàn bộ app nếu chỉ cần sửa lỗi nhỏ.
- Không đổi stack.
- Không hardcode path máy cá nhân nếu không cần.
- Không xóa dữ liệu user.
- Không lưu terminal output/log.
- Không commit/push nếu user chưa yêu cầu.
- Không build release nếu user chưa yêu cầu.
- Không chạy `npm run build`, `npm run build:win`, `npm run tauri build` nếu user chưa yêu cầu.
- Không tự ý thêm tính năng ngoài scope.

## 3. Quy tắc dev

Nếu chỉ sửa code thông thường, chạy:

```bash
npm run typecheck
npm run dev
```

Nếu task chỉ sửa tài liệu thì không cần chạy `npm run dev`.

Nếu `npm run dev` lỗi port `1420`, kiểm tra process đang chiếm port, không đổi port vội.

Nếu `npm run dev` lỗi `os error 740`, kiểm tra manifest Windows phải là `asInvoker`, không dùng `requireAdministrator`.

Không yêu cầu app chạy Administrator mặc định.

## 4. Quy tắc build

Chỉ khi user yêu cầu build mới chạy build.

Khi cần xóa build cũ trên Windows PowerShell:

```powershell
Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force build -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force .vite -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force node_modules\.vite -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force src-tauri\target -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force target -ErrorAction SilentlyContinue
```

Sau đó chạy:

```bash
npm install
npm run typecheck
npm run build:win
```

Không commit build artifacts:

- `*.exe`
- `*.msi`
- `*.pdb`
- `dist/`
- `build/`
- `target/`
- `src-tauri/target/`
- `node_modules/`

## 5. Quy tắc terminal

- Terminal phải mở đúng thư mục project đang chọn.
- Terminal không được tự lưu output/log.
- Stop terminal = gửi Ctrl+C hai lần bằng `\x03`, không kill/close terminal.
- Close terminal = đóng thật, cleanup frontend/backend, remove session, không rò rỉ memory.
- Context menu terminal phải i18n.
- Terminal màu project phải gắn với terminal tab, không lấy từ `activeProject` hiện tại.
- Không còn Terminal Admin. Nếu còn code Admin Terminal thì phải báo lại hoặc xóa nếu task yêu cầu.
- Terminal render phải giữ nguyên ANSI output, không trim/split/replace escape sequence.
- Không đặt padding trực tiếp trên `.xterm`.
- xterm `cols/rows` phải đồng bộ với PTY backend.

## 6. Quy tắc Git

- Git không tự load khi mở app hoặc chọn project.
- Git chỉ load khi user bấm Load Git/Refresh.
- Git load không được làm lag Terminal/Files/Settings.
- Nếu lỗi Git, hiển thị lỗi rõ, không hiển thị `[object Object]`.

## 7. Quy tắc Files/Editor

- File tab close/close all/close others phải hoạt động.
- i18n không được hiện raw key như `editor.close`.
- Markdown file cần hỗ trợ preview nếu tính năng đã có.
- Không làm hỏng mở file `.svg` dạng text nếu source đang hỗ trợ.

## 8. Quy tắc i18n

- UI chính không hardcode tiếng Anh nếu đang có hệ i18n.
- Khi thêm key mới, phải thêm cả `vi` và `en`.
- Sau khi sửa i18n, rà soát không còn raw key hiển thị trên UI.
- Settings đổi tiếng Việt/English phải phản ánh đúng.

## 9. Quy tắc bảo mật

Không commit secrets hoặc credential:

- `API_KEY`
- `SECRET`
- `TOKEN`
- `PASSWORD`
- `PRIVATE_KEY`
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GITHUB_TOKEN`
- `.env`
- `.env.*`

Không commit:

- `node_modules/`
- `dist/`
- `build/`
- `target/`
- `src-tauri/target/`
- `*.exe`
- `*.msi`
- `*.pdb`
- `*.AppImage`
- `*.deb`
- `*.rpm`

Backend đọc/ghi file phải giữ project boundary.

Không dùng `taskkill` bừa.

Không kill process ngoài phạm vi CliDesk.

Không bypass UAC.

## 10. Quy tắc GitHub

Trước khi commit:

```bash
git status
git diff --stat
git diff
```

Không commit build artifacts.

Commit message ngắn, rõ.

Không push nếu user chưa yêu cầu.

Không dùng `git push --force`.

## 11. Mẫu prompt task

Copy block này khi giao task cho AI coding agent:

```text
Bạn là AI coding agent đang làm việc trong dự án CliDesk.

Trước khi làm task này, hãy đọc:
- AI_AGENT_PROMPT_TEMPLATE.md
- AGENTS.md nếu tồn tại
- README.md nếu cần

Task:
[MÔ TẢ TASK Ở ĐÂY]

Yêu cầu:
1. Đọc source hiện tại trước khi sửa.
2. Không phỏng đoán nếu có thể kiểm tra.
3. Không build release nếu tôi chưa yêu cầu.
4. Sau khi sửa, chạy npm run typecheck.
5. Nếu task liên quan app runtime, chạy npm run dev.
6. Báo cáo cuối:
   - Nguyên nhân
   - File đã sửa
   - Cách sửa
   - Lệnh đã chạy
   - Kết quả test
   - Lỗi còn lại nếu có
```

## 12. Báo cáo cuối của agent

Mỗi lần hoàn thành task, agent phải báo cáo:

- Đã đọc file hướng dẫn nào.
- Nguyên nhân lỗi hoặc mục tiêu thay đổi.
- File đã sửa.
- Cách sửa.
- Lệnh đã chạy.
- Kết quả typecheck/dev/build nếu có.
- Có commit/push hay chưa.
- Lỗi còn lại nếu có.
