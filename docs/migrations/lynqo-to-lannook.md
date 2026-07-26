# 从 LYNQO 迁移到 LanNook

LanNook 是 LYNQO 的新名称。重命名不会改变项目的局域网传输定位，但会把新的配置、数据库、日志、浏览器存储和局域网发现记录写入 LanNook 命名空间。

## 升级时会自动迁移的内容

- 电脑端会将旧数据目录中的 `lynqo.db`、SQLite WAL/SHM 文件、`config.json` 和日志目录移入新目录，并改用 `lannook.db` 与 `lannook.log`。
- Windows 的目录从 `%APPDATA%\LYNQO` 迁入 `%APPDATA%\LanNook`；其他桌面系统从 `~/.config/LYNQO` 迁入 `~/.config/LanNook`。
- 浏览器会把旧的语言选择、协议确认、首次引导和手机稳定设备 ID 复制到新的 `lannook.*` 键。旧键会暂时保留，避免旧页面缓存影响本次升级。
- 旧的 `LYNQO LAN File Transfer` Windows 防火墙规则会继续被识别；新建规则会使用 `LanNook LAN File Transfer`。

## 不需要手动处理

- 不需要重新授权已有设备。
- 不需要搬运接收目录或传输记录。
- 不需要重新配置关闭行为或开机自启。

## 有意保留的兼容层

`com.lynqo.desktop` 仍保留为桌面包标识。它不会出现在界面或安装包名称中，但保留它可以让已安装的 LYNQO v26.1.7 用户通过原有更新链路原地升级到 LanNook；现在改动这个标识会造成系统将 LanNook 当作另一款应用并存安装。

局域网 mDNS 服务类型已由 `_lynqo._tcp.local.` 改为 `_lannook._tcp.local.`。使用二维码或完整连接地址的手机不受影响；依赖 mDNS 发现的设备应重新打开 LanNook 以刷新附近设备列表。

## 回退

迁移只在目标文件不存在时执行，不会覆盖已经存在的 LanNook 数据。若迁移失败，旧 LYNQO 文件会保留原处，应用会以新的空数据目录启动；请提交连接诊断和日志后再手动处理旧文件。
