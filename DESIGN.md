# Design Document: cloudcut-challenge

1.1 : ทำไมเลือก SQLx / SeaORM / Raw SQL?

เลือก Raw sql เพราะว่า ประเมิณแล้วว่าระบบน่าจะมีการ query ที่ซับซ้อน โจทย์กำหนดว่า "project detail returns full timeline" ของที่จะต้องดึงมาเเสดง Tracks, Clips, ClipEffects, Transitions, และ TextOverlays

============================================================

1.2 : จุดไหน normalize และจุดไหน denormalize?

Normallize : การแตกตารางเพื่อลดความซ้ำซ้อนของข้อมูล เชื่อมของต่างๆ ด้วยการเก็บ FK ( Users, Workspaces, Projects, Tracks, Clips )

Denormalize : ยอมเก็บข้อมูลซ้ำซ้อนเพื่อให้มันอยู่ที่เดียวเราสามารถแตกมันไปเป็นอีกTable
Table : transform JSONB ({"x": 10, "y": 20, "scale": 1.5})

============================================================

1.3 : Soft delete strategy ทำอย่างไร ?

ใช้ท่ามาตรฐานเลยครับ deleted_at ใช้คอลัมน์ deleted_at TIMESTAMPTZ NULL

Tip : ถ้าเรามี query ที่มี deleted_at สามารถสร้าง idx เเบบนี้เพื่อมาหาของที่ยังไม่ลบให้เร็วขึ้นได้

CREATE INDEX idx_clips_active ON clips (project_id) WHERE deleted_at IS NULL;

============================================================

1.4 : Cascade cleanup ทำอย่างไร ?
ใช้หลักการทั่วไปเลยครับใช้ของที่สำคัญที่สุดเป็นแกนหลัก เช่น
ถ้า project โดนลบ Tracks, Clips, Effects โดนลบไปด้วย
project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE

============================================================

1.5 : ทำไม clip position เก็บเป็น track_position_ms?
วิดิโอมันต้องการความเเม่นยำระดับเฟรม เก็บ float , double จะเจอปัญหา Floating-point error (เช่น 0.1 + 0.2 = 0.30000000000000004) ส่งผลให้เกิดช่องว่างขนาด 1 เฟรม (Black frame) หรือคลิปเกยกันตอน Render

============================================================

1.6 : OperationLog จะโตเร็ว จะ archive หรือ partition อย่างไร ?

มันเป็น Operation Log db บวมอยู่เเล้วก็จะกำหนด policy ว่าเก็บไว้ไม่เกิน 1 เดือน / 1 อาทิตย์ ตามที่ไหว

สร้าง db partiton มาเพื่อเก็บของเเต่ละเดือน เช่น operation_logs_2026_05 , operation_logs_2026_06 ,

============================================================

1.7 : Estimate rows สำหรับ 1,000 users × 10 projects × 30 clips

ข้อนี้ใช้ AI ช่วยคำนวณเพราะจริงๆ ข้อมูลมัน Fix อยู่เเล้วครับ user , project , clip สิ่งที่ต่างคือ Tracks , Clip Effects ถ้ารู้ค่าเฉลี่ยว่า user ใช้ประมาณกี่ track ต่อวิดิโอก็ตอบได้เเล้ว

ตาราง Users: มีผู้ใช้ 1,000 คน ตรงๆ ตัว = 1,000 แถว
ตาราง Projects: ผู้ใช้ 1,000 คน × คนละ 10 โปรเจกต์ = 10,000 แถว
ตาราง Tracks (แถวชั้นเลเยอร์บน Timeline): สมมติว่าโดยทั่วไป วิดีโอ 1 โปรเจกต์ จะมีแทร็กมาตรฐานประมาณ 4 ชั้น (เช่น แทร็กภาพหลัก, แทร็กภาพซ้อน, แทร็กเสียงพูด, แทร็กเพลงประกอบ)

คิดเป็น: 10,000 โปรเจกต์ × 4 แทร็ก = 40,000 แถว
ตาราง Clips (ตัวคลิปที่โดนตัดเป็นชิ้นๆ): โจทย์บอกว่า 1 โปรเจกต์มี 30 คลิป
คิดเป็น: 10,000 โปรเจกต์ × 30 คลิป = 300,000 แถว
ตาราง Clip Effects (เอฟเฟกต์ฟิลเตอร์): เราสมมติเพิ่มเผื่อไว้ว่าเฉลี่ยแล้ว ทุกๆ 1 คลิป คนจะใส่เอฟเฟกต์ (เช่น ปรับแสง หรือใส่ฟิลเตอร์) 1 อย่าง
คิดเป็น: 300,000 คลิป × 1 เอฟเฟกต์ = 300,000 แถว

============================================================

2.1 ทำไมเลือก Axum / Actix ?

Axum เป็นเทคโนโลยีใหม่กว่า , เด่นด้าน type safety มากกว่า ,
คนนิยมกว่า = มี ecosystem มากกว่า เวลาเกิดปัญหาหาคำตอบง่่ายกว่า
เมื่อ Research เพิ่มทำให้ทราบว่า Axum ถูกสร้างบน Tokio และ Tower โดยตรง ซึ่งทำให้ abstraction หลักทั้งหมดใช้แนวคิดเดียวกับ ecosystem กลางของ Rust

============================================================

2.2 ทำไมเลือก SQLx / SeaORM?

อย่า่งเเรกเลยเราใช้ rawSQl มันต้องใช้คู่กับ SQLx อยู่เเล้วครับ ถ้าจะเขียน query ที่ซับซ้อนเองจะใช้ orm ทำไม อย่างที่สองเมื่่อ research เพิ่มทำให้ร้ว่า SQLx เป็น Compile-time verified SQL หมายความว่าตัวคอมไพเลอร์ Rust จะวิ่งไปตรวจเช็กกับฐานข้อมูลจริงใน Docker ทันทีว่าเราพิมพ์คำสั่ง SQL ผิด หรือสะกดชื่อคอลัมน์ผิดหรือไม่ตั้งแต่ตอนคัดลอกโค้ด ป้องกันบั๊กหลุดไปรันไทม์ ในขณะที่ SeaORM (ที่เป็น ORM เต็มรูปแบบ) จะสร้าง Layer ครอบหนาเกินไป ทำให้รีดประสิทธิภาพได้ไม่สุด และยากต่อการเขียนคิวรีลึกๆ

============================================================

2.3 Cursor-based pagination ทำงานอย่างไร ?
จะใช้ค่าเพื่อกำหนดจุดที่จะเริ่มค้นหา เช่น
< '2026-05-17 10:00:00' หรือ > '2026-05-17 10:00:00'
จากที่จะต้อง scan ทั้งหมดก็ไม่ต้องเเล้ว

============================================================

2.4 Presigned upload flow ทำงานอย่างไร ?

Frontend ส่งข้อมูลไฟล์ที่ต้องการ upload มาให้ backend ก่อน เช่น file name, size หรือ mime-type

Backend จะ validate เบื้องต้น เช่น auth , quota , file size , file type

จากนั้น backend จะใช้ AWS credentials ฝั่ง server สร้าง “Presigned URL” หรือ URL ชั่วคราวที่มีลายเซ็นดิจิทัลและวันหมดอายุ

Frontend จะนำ URL นี้ยิง upload ไฟล์ตรงเข้า S3 ได้เลย โดยไม่ต้องผ่าน backend อีกที

back-end ต้องถือ AWS Access Key ของ S3 ที่ถูกต้องด้วยถึงจะทำงานได้

============================================================

2.5 ทำไมไม่ upload file ผ่าน backend โดยตรง ?

ไฟล์วิดิโอมันขนาดใหญ่ ถ้ามันต้องยิงต่อไป s3 อีก เปลือง bandwith , resource จากข้อ 2.4 เราทำ presigned เเล้วก็ เซ็นกลับไปให้ frontend เเล้วใช้ user ยิงตรงได้ ลดภาระ back-end ได้ผลลัพธ์เหมือนเดิม

============================================================

2.6 Batch clip operation ควร atomic transaction หรือ partial success ?

Atomic Transaction (All-or-Nothing) เพราะมันเป็นงานที่ต้องทำสำเร็จทั้งหมดถึงจะถูกต้องเเละเพื่อรักษาความถูกต้องของข้อมูลด้วย เช่น เราจะ group 5 คลิป เเต่ถ้าใช้ partial เเล้วสำเร็จเเค่ 4/5 ก็ไม่ถูกต้อง

============================================================

2.7 API versioning จะจัดการอย่างไรถ้ามี breaking change ?

จัดการผ่าน URL Path-based Versioning
(เช่น /api/v1/projects และ /api/v2/projects)

============================================================

2.8 Authorization layer วางไว้ที่ middleware, extractor หรือ service layer ?

Middleware สำหรับตรวจเบื้องต้นเช็คเบื้องต้นว่ามี JWT จริงไหม

# Extractor / Service Layer สำหรับตรวจสิทธ์ในการเข้าถึง resource เช่นมี JWT เเต่พยายามจะดูงานใน workspace ของคนอื่น

2.9 Error handling strategy เป็นอย่างไร ?

Centralized Monolithic Error Mapping AppError

pub enum AppError { #[error("Unauthorized")]
Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),

}

============================================================

3.1 ทำไมเลือก Redis Streams / Apalis / PostgreSQL job table?

ไม่ได้ใช้ Redis Streams เเต่ใช้ Redis List เพราะระบบของเราต้องการความเร็วและความเรียบง่าย การใช้ LPUSH และ BRPOP ช่วยให้จัดการง่ายกว่ามาเขียนเอง

เราใช้ตาราง export_jobs และ assets เป็น Source of Truth เพราะ Redis เป็นเพียง Message Broker ชั่วคราว การมีตารางใน PostgreSQL ช่วยให้เราเก็บสถานะงานแบบถาวร (Persistence) และเก็บ Metadata ของวิดีโอที่สกัดมาได้ เพื่อให้ Frontend สามารถ Query ข้อมูลกลับมาแสดงผลได้ตลอดเวลา

============================================================

3.2 Retry และ dead-letter queue ทำงานอย่างไร ?

Exponential Backoff Retry : เมื่อ Job เกิดข้อผิดพลาด (เช่น Network Error หรือ ffmpeg ขัดข้องชั่วคราว) Worker จะไม่รันใหม่ทันที แต่จะคำนวณเวลาหน่วงตามสูตร 2 ยกกำลัง attempts วินาที (1s, 2s, 4s, 8s) เพื่อรอให้ทรัพยากรกลับมาพร้อมใช้งานและลดภาระระบบ

Maximum Attempts : จำกัดการลองใหม่ไว้ที่สูงสุด 4 ครั้ง เพื่อป้องกันการใช้ทรัพยากรไปกับงานที่ไม่สามารถทำสำเร็จได้จริงๆ

หากงานล้มเหลวครบตามกำหนด Job นั้นจะถูกย้ายออกจากคิวหลักไปยัง queue:video_pipeline:dead_letter ทันที พร้อมเก็บข้อมูล Job ต้นฉบับ และ Error Message ล่าสุดไว้ในรูปแบบ JSON

============================================================

3.3 Idempotency ป้องกัน duplicate processing อย่างไร ?

เราสร้าง idempotency_key จาก UUIDV7 ตั้งแต่ฝั่ง API

ในฐานข้อมูล ตาราง export_jobs มีคอลัมน์ idempotency_key (Unique) ซึ่งจะป้องกันไม่ให้เกิดการ Render ซ้ำซ้อนหาก Frontend ส่ง Request เดิมมาเบิ้ล หรือกรณี Network Retr

============================================================

3.4 ffmpeg CLI มีข้อดีข้อเสียอะไร ?

ข้อดี : การเรียกผ่าน std::process::Command ทำให้ ffmpeg รันแยกเป็น OS Process ข้อดีคือถ้า ffmpeg แคช (Crash) จะไม่ทำให้ Worker ของเราตายไปด้วย และไม่มีปัญหาเรื่อง Memory Leak ภายใน Rust

ข้อเสีย : การจัดการ Error ทำได้ลำบาก (ต้องอ่านจาก stderr) และการ Spawn process ใหม่มี Overhead เล็กน้อย

============================================================

3.5 ถ้า video ยาว 30 นาที memory และ temp file จะจัดการอย่างไร ?

Memory : เราใช้การ Streaming ผ่าน Presigned URL ทำให้ ffmpeg อ่านข้อมูลจาก Network โดยตรง ไม่ต้องโหลดวิดีโอ 30 นาทีลง RAM

Temp Files : เราใช้เทคนิค Segmented Rendering ใน export_pipeline.rs โดยหั่นเฉพาะส่วนที่ใช้จริงมาเป็นชิ้นเล็กๆ แล้วใช้คำสั่ง remove_dir_all ล้างทิ้งทันทีหลังจบงาน

============================================================

3.6 Export job cancel ทำอย่างไร ?

เราสามารถเก็บ Child Process ID (PID) ของ ffmpeg ไว้ใน Redis เมื่อมีการ Cancel จาก API เราจะส่งสัญญาณ SIGKILL ไปยัง PID นั้น และอัปเดตสถานะใน DB เป็น Cancelled

============================================================

3.7 จะ scale worker หลายเครื่องอย่างไร?

เนื่องจากเราใช้ BRPOP ซึ่งเป็นคำสั่งแบบ Atomic ของ Redis งาน 1 ชิ้นจะถูกดึงไปโดย Worker เพียงตัวเดียวเสมอ (Competing Consumers) เราจึงสามารถเปิด Worker เพิ่มได้ทันที (Horizontal Scale) โดยไม่ต้องแก้ไข Code

============================================================

3.8 Cost estimation สำหรับ 1 export job 1080p ความยาว 5 นาที

Compute : ใช้ CPU 100% ประมาณ 2-3 นาที (บนเครื่อง t3.medium)

Storage/Transfer : ไฟล์ผลลัพธ์ขนาดประมาณ 200MB (Bitrate 5Mbps)

ต้นทุนรวม : ประมาณ 0.02$ − 0.05$ ต่อการ Export

============================================================
