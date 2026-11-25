# Glosarium istilah UI

Istilah inti untuk 14 terjemahan WangunGIS. Daftar ini disusun Claude (Opus) pada 2026-11-25 sebagai draf mesin. Penutur asli yang juga engineer sipil atau arsitek diminta meninjaunya lewat format umpan balik beta. Bila ada koreksi, ubah glosarium ini dulu, lalu katalognya.

## Aturan untuk semua bahasa
- **Tetap seperti aslinya, tidak diterjemahkan:**
  - Nama tool **Push/Pull** dan **Follow Me**, serta tampilan **Iso**.
  - Huruf shortcut di dalam kurung, seperti `(P)` atau `(Space)`. `Cmd/Ctrl` dan nama tombol `Esc`, `Shift`, `Enter` juga tetap.
  - Nama format dan ekstensi berkas (`.hew`, `.wangun`, `.las`, `.obj`), EPSG, LAS, satuan (`m`, `mm`, `ft`), contoh ketikan (`5;4`, `30d`, `x10`, `/5`, `8s`), dan placeholder `{0}`, `{1}`.
  - Nama-nama di atas boleh diberi padanan dalam kurung bila perlu, misalnya `Push/Pull (押し出し)`. Cukup pada tooltip, bukan pada label pendek.
- **Gaya bahasa:** UI ringkas, sopan, dan netral. Pakai bentuk perintah yang lazim di perangkat lunak bahasa itu, misalnya bentuk kamus atau suru di Jepang, 하세요/합니다 di Korea, imperatif di Eropa, dan `कृपया` dihindari di Hindi.
- **Kata yang tidak ada di daftar:** pakai istilah CAD atau BIM yang lazim di bahasa itu. Jangan menyalin istilah merek perangkat lunak lain.
- **Arab:** tulis teks Arab biasa. Tata letak tetap kiri-ke-kanan (lihat `SPEC.md`).

## Istilah inti
Urutan kolom: id | en | zh-Hans | zh-Hant | ja | ko | ar | es | pt-BR | fr | de | ru | tr | vi | th | hi

| id | en | zh-Hans | zh-Hant | ja | ko | ar | es | pt-BR | fr | de | ru | tr | vi | th | hi |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| face | face | 面 | 面 | 面 | 면 | وجه | cara | face | face | Fläche | грань | yüz | mặt | หน้า | फलक |
| tepi | edge | 边 | 邊 | エッジ | 모서리 | حافة | arista | aresta | arête | Kante | ребро | kenar | cạnh | ขอบ | किनारा |
| titik ujung | endpoint | 端点 | 端點 | 端点 | 끝점 | نقطة طرفية | extremo | extremidade | extrémité | Endpunkt | конечная точка | uç nokta | điểm đầu mút | จุดปลาย | अंत बिंदु |
| titik tengah | midpoint | 中点 | 中點 | 中点 | 중점 | نقطة المنتصف | punto medio | ponto médio | milieu | Mittelpunkt | середина | orta nokta | trung điểm | จุดกึ่งกลาง | मध्य बिंदु |
| region | region | 区域 | 區域 | 領域 | 영역 | منطقة | región | região | région | Bereich | область | bölge | vùng | พื้นที่ | क्षेत्र |
| solid | solid | 实体 | 實體 | ソリッド | 솔리드 | مجسم | sólido | sólido | solide | Volumenkörper | тело | katı | khối đặc | ของแข็ง | ठोस |
| grup | group | 组 | 群組 | グループ | 그룹 | مجموعة | grupo | grupo | groupe | Gruppe | группа | grup | nhóm | กลุ่ม | समूह |
| komponen | component | 组件 | 元件 | コンポーネント | 컴포넌트 | مكوّن | componente | componente | composant | Komponente | компонент | bileşen | thành phần | คอมโพเนนต์ | घटक |
| pecah | explode | 分解 | 分解 | 分解 | 분해 | تفكيك | descomponer | explodir | éclater | Auflösen | разбить | dağıt | phân rã | แยก | विघटित करें |
| tag | tag | 标签 | 標籤 | タグ | 태그 | وسم | etiqueta | etiqueta | étiquette | Tag | тег | etiket | thẻ | แท็ก | टैग |
| outliner | outliner | 大纲 | 大綱 | アウトライナー | 아웃라이너 | المخطط | esquema | estrutura | structure | Gliederung | структура | ana hat | dàn ý | โครงร่าง | रूपरेखा |
| point cloud | point cloud | 点云 | 點雲 | 点群 | 포인트 클라우드 | سحابة نقاط | nube de puntos | nuvem de pontos | nuage de points | Punktwolke | облако точек | nokta bulutu | đám mây điểm | พอยต์คลาวด์ | पॉइंट क्लाउड |
| titik awan | cloud point | 点云点 | 點雲點 | 点群の点 | 클라우드 점 | نقطة سحابة | punto de la nube | ponto da nuvem | point du nuage | Wolkenpunkt | точка облака | bulut noktası | điểm đám mây | จุดคลาวด์ | क्लाउड बिंदु |
| snap | snap | 捕捉 | 鎖點 | スナップ | 스냅 | التقاط | ajuste | encaixe | accrochage | Fang | привязка | yakalama | bắt điểm | สแนป | स्नैप |
| tile | tile | 瓦片 | 圖磚 | タイル | 타일 | بلاطة | tesela | bloco | tuile | Kachel | тайл | karo | ô | ไทล์ | टाइल |
| origin proyek | project origin | 项目原点 | 專案原點 | プロジェクト原点 | 프로젝트 원점 | أصل المشروع | origen del proyecto | origem do projeto | origine du projet | Projektursprung | начало координат проекта | proje orijini | gốc dự án | จุดกำเนิดโปรเจกต์ | प्रोजेक्ट मूल बिंदु |
| potong area | slice area | 裁切区域 | 裁切區域 | 範囲切り出し | 영역 자르기 | قص منطقة | recortar área | recortar área | découper une zone | Bereich zuschneiden | обрезка области | alanı kes | cắt vùng | ตัดพื้นที่ | क्षेत्र काटें |
| garis panduan | guide | 参考线 | 參考線 | ガイド | 가이드 | خط إرشادي | guía | guia | guide | Hilfslinie | направляющая | kılavuz | đường gióng | เส้นไกด์ | मार्गदर्शक रेखा |
| sumbu | axis | 轴 | 軸 | 軸 | 축 | محور | eje | eixo | axe | Achse | ось | eksen | trục | แกน | अक्ष |
| ukur | tape measure | 测量 | 測量 | メジャー | 줄자 | شريط القياس | cinta métrica | trena | mètre ruban | Maßband | рулетка | şerit metre | thước dây | เทปวัด | मापने का फीता |
| cat | paint | 涂色 | 塗色 | ペイント | 페인트 | طلاء | pintar | pintar | peindre | Malen | покраска | boya | tô màu | ระบายสี | रंग भरें |
| pilih | select | 选择 | 選取 | 選択 | 선택 | تحديد | seleccionar | selecionar | sélectionner | Auswählen | выбор | seç | chọn | เลือก | चुनें |
| pindah | move | 移动 | 移動 | 移動 | 이동 | نقل | mover | mover | déplacer | Verschieben | перемещение | taşı | di chuyển | ย้าย | स्थानांतरित करें |
| putar | rotate | 旋转 | 旋轉 | 回転 | 회전 | تدوير | rotar | girar | pivoter | Drehen | поворот | döndür | xoay | หมุน | घुमाएँ |
| skala | scale | 缩放 | 縮放 | 尺度 | 크기 조정 | تحجيم | escalar | escala | mettre à l'échelle | Skalieren | масштаб | ölçekle | co giãn | ย่อขยาย | स्केल करें |
| hapus | erase | 擦除 | 擦除 | 消去 | 지우기 | مسح | borrar | apagar | effacer | Radieren | ластик | sil | xóa | ลบ | मिटाएँ |
| solid baru | new solid | 新实体 | 新實體 | 新しいソリッド | 새 솔리드 | مجسم جديد | sólido nuevo | novo sólido | nouveau solide | neuer Volumenkörper | новое тело | yeni katı | khối đặc mới | ของแข็งใหม่ | नया ठोस |
| tampilan atas / depan / kanan | top / front / right view | 顶 / 前 / 右 | 頂 / 前 / 右 | 上 / 正面 / 右 | 위 / 앞 / 오른쪽 | أعلى / أمام / يمين | superior / frontal / derecha | superior / frontal / direita | dessus / face / droite | Oben / Vorne / Rechts | сверху / спереди / справа | üst / ön / sağ | trên / trước / phải | บน / หน้า / ขวา | ऊपर / सामने / दाएँ |
