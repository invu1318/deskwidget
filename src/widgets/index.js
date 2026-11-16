// 패널 위젯 등록부. src/widgets/kinds/ 안의 파일 하나가 위젯 종류 하나다.
//
// 각 파일은 아래 모양의 객체를 default로 내보낸다.
//   kind      앱(Rust WidgetKind)의 종류 이름. 예: 'calendar', 'worldclock'
//   label     설정창에 보이는 이름
//   note      추가 화면에 보이는 한 줄 설명
//   defaults  options의 기본값
//   fields    설정창이 그릴 입력 칸 목록. 각 칸은 { key, label, type, ... }
//             type: 'text' | 'number'(min,max) | 'select'(options: [[값, 표시]], 값은 문자열)
//                 | 'toggle' | 'date'(값 'YYYY-MM-DD') | 'file'(경로 + 찾기 버튼)
//                 | 'list'(item: [칸...], addLabel) — 값은 객체 배열이고 item 칸은 list를 뺀 나머지 type
//             공통 선택 항목: placeholder, hint, default(list item 칸의 새 항목 기본값)
//   css       이 위젯의 스타일. 선택자는 반드시 `.k-<kind>`로 시작한다.
//   mount(root, ctx)  root(#panel, class="k-<kind>") 안에 화면을 그린다. 정리 함수를 돌려준다.
//             ctx.options   defaults에 저장된 값을 덮은 설정값
//             ctx.data      이 위젯이 저장해 둔 내용(없으면 null)
//             ctx.saveData(data)  내용 저장. 설정이 바뀌어 다시 mount될 때도 이어진다.
//             ctx.widget    위젯 전체 정보(id, name 등)
//             ctx.isPaused()  전체화면 앱이 떠 있으면 true. 주기 작업은 이때 건너뛴다.
//   설정이 바뀌면 정리 함수를 부른 뒤 root를 비우고 mount를 다시 부른다.
const modules = import.meta.glob('./kinds/*.js', { eager: true });

export const kinds = Object.fromEntries(Object.values(modules).map((m) => [m.default.kind, m.default]));

export const optionsOf = (widget) => ({ ...kinds[widget.kind]?.defaults, ...(widget.options ?? {}) });
