<script setup lang="ts">
import { useRouter } from "vue-router";
import { ArrowLeft, Download, Send, ShieldCheck, Wifi, Wrench } from "lucide-vue-next";

const router = useRouter();
</script>

<template>
  <div class="help-page">
    <button class="back-btn" @click="router.back()">
      <ArrowLeft :size="16" /> 返回
    </button>

    <header class="page-header">
      <h1>使用帮助</h1>
      <p>局域网内设备授权、发送与接收文件的操作说明。</p>
    </header>

    <section class="help-card">
      <div class="section-heading"><Wifi :size="18" /><h2>连接手机</h2></div>
      <ol>
        <li>在电脑端确认顶部状态显示“运行中”。</li>
        <li>手机与电脑连接同一个 Wi-Fi；用手机扫描“连接设备”中的二维码，或输入二维码中的完整地址。</li>
        <li>在电脑端“设备”页找到手机后，点击“授权”。手机页面会在几秒内自动更新。</li>
      </ol>
    </section>

    <section class="help-card">
      <div class="section-heading"><Send :size="18" /><h2>发送文件</h2></div>
      <p>手机端可直接把文件上传到电脑，或选择一台已授权且在线的手机作为目标设备。</p>
      <p>选择其他手机时，文件会先安全上传到电脑，随后等待目标手机确认下载；“已上传，等待目标设备接收”不等于对方已经收到。</p>
    </section>

    <section class="help-card">
      <div class="section-heading"><Download :size="18" /><h2>接收文件</h2></div>
      <p>电脑接收的文件保存在“设置”中配置的接收文件夹。手机接收时，请保持 LYNQO 页面打开；收到请求后选择“接受”才会开始下载。</p>
    </section>

    <section class="help-card">
      <div class="section-heading"><Wrench :size="18" /><h2>连接不上或看不到设备</h2></div>
      <ul>
        <li>不要在手机上输入 <code>localhost</code> 或 <code>127.0.0.1</code>；应使用电脑显示的局域网 IP 地址。</li>
        <li>确认 Windows 防火墙允许 LYNQO 使用“专用网络”，并关闭会隔离客户端的访客 Wi-Fi。</li>
        <li>设备离线通常表示手机页面已关闭、休眠或网络切换。重新打开已授权页面即可恢复在线。</li>
        <li>每台浏览器会用稳定设备标识连接；刷新页面不会再创建新的授权卡片。</li>
      </ul>
    </section>

    <section id="privacy" class="help-card">
      <div class="section-heading"><ShieldCheck :size="18" /><h2>隐私与权限</h2></div>
      <p>文件传输在当前局域网和本机接收目录内完成，不会上传到公共云端。网页端只能查看和操作与当前已授权设备有关的传输，不能访问电脑本地文件或管理其他设备。</p>
      <p class="legal-links">
        <RouterLink to="/legal/privacy">隐私说明</RouterLink>
        <RouterLink to="/legal/terms">使用协议</RouterLink>
        <RouterLink to="/legal/disclaimer">免责声明</RouterLink>
      </p>
    </section>
  </div>
</template>

<style scoped>
.help-page { max-width: 760px; margin: 0 auto; padding: 28px 32px 48px; }
.back-btn { display: inline-flex; align-items: center; gap: 6px; border: 0; background: transparent; color: var(--color-text-secondary); cursor: pointer; padding: 6px 0; }
.page-header { margin: 18px 0 24px; }
.page-header h1 { margin: 0; font-size: var(--text-2xl); color: var(--color-text-primary); }
.page-header p { margin: 8px 0 0; color: var(--color-text-secondary); }
.help-card { padding: 20px 22px; margin-bottom: 14px; border: 1px solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-surface-card); box-shadow: var(--shadow-card); color: var(--color-text-secondary); line-height: 1.7; }
.section-heading { display: flex; align-items: center; gap: 8px; color: var(--color-text-primary); }
.section-heading svg { color: var(--color-brand-primary); }
.section-heading h2 { margin: 0; font-size: var(--text-lg); }
.help-card p { margin: 12px 0 0; }
.help-card ol, .help-card ul { margin: 12px 0 0; padding-left: 22px; }
.help-card code { padding: 1px 4px; border-radius: 4px; background: var(--color-surface-inset); color: var(--color-text-primary); }
.legal-links { display: flex; flex-wrap: wrap; gap: 14px; }
.legal-links a { color: var(--color-brand-primary); text-decoration: none; }
.legal-links a:hover { text-decoration: underline; }
@media (max-width: 640px) { .help-page { padding: 20px 16px 36px; } }
</style>
