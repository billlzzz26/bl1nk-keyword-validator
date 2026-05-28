# 🛡️ bl1nk Keyword Validator & Search Infrastructure

**bl1nk Keyword Validator** คือโครงสร้างพื้นฐานสำหรับการจัดการคำสำคัญ (Keyword Infrastructure) ทำหน้าที่เป็น "Knowledge Backbone" ให้กับระบบเอเจนต์อัจฉริยะ โดยมุ่งเน้นที่ความถูกต้องของข้อมูลและการค้นหาที่แม่นยำสูง รองรับการทำงานทั้งในรูปแบบไลบรารี (Library) และเครื่องมือบรรทัดคำสั่ง (CLI)

## 🎯 วิสัยทัศน์ (Vision)
โปรเจกต์นี้มุ่งเน้นการเป็น **ระบบค้นหาและตรวจสอบสคีมา (Search & Validation Engine)** ที่รองรับการทำงานร่วมกับระบบฐานข้อมูลคำสำคัญขนาดใหญ่ โดยเน้นประสิทธิภาพการค้นหาภาษาไทยอัจฉริยะและสถาปัตยกรรมข้อมูลที่ยืดหยุ่น เพื่อสนับสนุนการทำงานของ AI Agents ในการเข้าถึงข้อมูลที่ถูกต้อง

## 🚀 ฟีเจอร์เด่น (Key Features)

- 🔍 **Smart Search (BM25)**: ระบบค้นหาที่คำนวณคะแนนความเกี่ยวข้อง (Relevance Scoring) ช่วยให้เจอผลลัพธ์ที่ตรงใจที่สุด
- 🇹🇭 **Thai Language Optimization**: รองรับระบบ **Thai Bigram** และ **Tone-mark Insensitive Search** (ถอดวรรณยุกต์) ทำให้ค้นหาภาษาไทยได้แม่นยำแม้พิมพ์ไม่ครบ
- ✅ **Strict Schema Validation**: ตรวจสอบโครงสร้างข้อมูลอย่างเข้มงวด รวมถึงการเช็ค Broken Links และ Namespace Isolation
- 📂 **Multi-format Support**: รองรับทั้งไฟล์ **JSON** และ **YAML** เพื่อความสะดวกในการจัดการข้อมูล
- 📡 **Project-wide Scanning**: คำสั่ง `Scan` สำหรับการกวาดหาและตรวจสอบไฟล์ Registry ทั่วทั้งโปรเจกต์โดยอัตโนมัติ
- 🛠️ **Modern Architecture**: พัฒนาด้วย Rust (Edition 2024), ใช้ `Clap` v4, และระบบ Log ด้วย `Tracing`

## 🏗️ การติดตั้งและสร้าง (Build)

### สร้างไฟล์รันหลัก (Optimized)
```bash
cargo build --release
# ผลลัพธ์: target/release/keyword-registry
```

### ติดตั้งเป็นคำสั่งในเครื่อง
```bash
cargo install --path cli
# เรียกใช้ด้วยคำสั่ง: keyword-registry
```

## 📖 วิธีการใช้งาน (Usage)

### คำสั่ง CLI หลัก

#### 1. ค้นหาคำสำคัญ (Search)
```bash
# ค้นหาแบบปกติ (รองรับไทย/อังกฤษ)
keyword-registry search "visual-story"

# ค้นหาและแสดงผลเป็น JSON เพื่อนำไปใช้ต่อในโปรแกรมอื่น
keyword-registry search "ภาพเรื่อง" --json
```

#### 2. สแกนและตรวจสอบทั้งโปรเจกต์ (Scan)
```bash
# สแกนหาไฟล์ .json/.yaml และตรวจสอบความถูกต้อง
keyword-registry scan --dir . --ignore "**/node_modules/**,**/target/**"
```

#### 3. ตรวจสอบความถูกต้อง (Validate)
```bash
# ตรวจสอบทั้งไฟล์
keyword-registry validate

# ตรวจสอบเฉพาะรายการที่ระบุ
keyword-registry validate item-id --group projects
```

#### 4. การจัดการข้อมูล (CRUD)
```bash
# เพิ่มรายการใหม่ (รองรับ @ไฟล์)
keyword-registry add projects @new-entry.json

# แก้ไขข้อมูลรายการ
keyword-registry edit item-id --group projects --field description --value "ใหม่"
```

### การใช้งานในฐานะไลบรารี (Library Usage)

```rust
use bl1nk_keyword_core::{load_registry, KeywordSearch, Validator};

// โหลดข้อมูล (รองรับ JSON/YAML อัตโนมัติจากนามสกุลไฟล์)
let registry = load_registry("keyword-registry.yaml")?;

// เริ่มระบบค้นหาอัจฉริยะ
let search = KeywordSearch::new(registry.clone());
let results = search.search("ค้นหาคำนี้");

// เริ่มระบบตรวจสอบความถูกต้อง
let validator = Validator::new(registry);
validator.validate_registry()?;
```

## 📂 โครงสร้างโปรเจกต์ (Project Structure)

```
bl1nk-keyword-validator/
├── core/                # ไลบรารีหลัก (Logic, Search, Validation)
│   └── src/
│       ├── lib.rs       # Entry point & Persistence
│       ├── search.rs    # BM25 & Thai NLP Search
│       ├── validator.rs # Validation Logic
│       └── schema.rs    # Data Models
├── cli/                 # เครื่องมือบรรทัดคำสั่ง (Interface)
│   └── src/main.rs      # CLI Implementation
└── scripts/             # สคริปต์ช่วยพัฒนา
```

## 📝 รหัสข้อผิดพลาด (Error Codes)

- `DUPLICATE_ID`: พบ ID ซ้ำในกลุ่มเดียวกัน
- `DUPLICATE_ALIAS`: พบชื่อแฝงซ้ำในระบบ
- `BROKEN_RELATIONSHIP`: การอ้างอิง ID (relatedIds) ไม่ถูกต้อง
- `INVALID_PATTERN`: ข้อมูลไม่ตรงตามรูปแบบ Regex ที่กำหนด
- `ALIAS_TOO_SHORT/LONG`: ความยาวชื่อแฝงไม่เป็นไปตามกฎ

## 👨‍💻 ผู้พัฒนา
**อาจารย์ (Dollawatt)** และทีมงาน **bl1nk**

## ⚖️ สัญญาอนุญาต (License)
MIT License
