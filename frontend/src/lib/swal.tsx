import Swal from "sweetalert2";
import withReactContent from "sweetalert2-react-content";

const MySwal = withReactContent(Swal);

/* 
  Toast Configuration สำหรับการแจ้งเตือนขนาดเล็ก (Production-ready)
*/
export const toast = MySwal.mixin({
  toast: true,
  position: "top-end",
  showConfirmButton: false,
  timer: 3000,
  timerProgressBar: true,
  background: "#18181b" /* zinc-900 ให้เข้ากับธีมมืด */,
  color: "#ffffff",
  didOpen: (toast) => {
    toast.addEventListener("mouseenter", Swal.stopTimer);
    toast.addEventListener("mouseleave", Swal.resumeTimer);
  },
});

/* 
  Alert สำหรับข้อผิดพลาดหรือยืนยันการกระทำ
*/
export const alert = {
  error: (title: string, message: string) => {
    return MySwal.fire({
      icon: "error",
      title: <span className="text-xl font-bold">{title}</span>,
      html: <span className="text-sm text-zinc-400">{message}</span>,
      background: "#18181b",
      color: "#ffffff",
      confirmButtonColor: "#3f3f46" /* zinc-700 */,
      customClass: {
        popup: "rounded-lg border border-zinc-800 shadow-2xl",
        confirmButton: "px-6 py-2 text-sm font-medium rounded-md",
      },
    });
  },
  success: (title: string) => {
    return MySwal.fire({
      icon: "success",
      title: <span className="text-xl font-bold">{title}</span>,
      background: "#18181b",
      color: "#ffffff",
      showConfirmButton: false,
      timer: 1500,
      customClass: {
        popup: "rounded-lg border border-zinc-800 shadow-2xl",
      },
    });
  },
};
