package org.djimic.control

import android.app.Activity
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbDeviceConnection
import android.hardware.usb.UsbManager
import android.os.Build
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Gives the Rust device layer access to DJI receivers. Android apps can't
 * scan USB themselves: each device needs the user's permission, then
 * UsbManager opens it and hands back a file descriptor, which Rust uses with
 * the same protocol code as on desktop.
 */
@TauriPlugin
class UsbPlugin(private val activity: Activity) : Plugin(activity) {
    private val usb by lazy { activity.getSystemService(Context.USB_SERVICE) as UsbManager }

    /** Open connections, by device name (/dev/bus/usb/…). */
    private val open = mutableMapOf<String, UsbDeviceConnection>()

    /** Devices we already asked permission for, so the dialog isn't repeated. */
    private val asked = mutableSetOf<String>()

    /**
     * Every attached DJI receiver the app may use, opened. Asks for
     * permission (once per plug-in) for the ones it may not use yet.
     */
    @Command
    fun openDevices(invoke: Invoke) {
        val attached = usb.deviceList
        // Close connections to receivers that were unplugged.
        open.keys.filter { it !in attached.keys }.forEach { name ->
            open.remove(name)?.close()
        }
        asked.retainAll(attached.keys)

        val list = JSArray()
        for ((name, dev) in attached) {
            if (dev.vendorId != DJI_VENDOR_ID || dev.productId !in RECEIVER_PRODUCT_IDS) continue
            if (!usb.hasPermission(dev)) {
                if (asked.add(name)) requestPermission(dev)
                continue
            }
            val conn = open[name] ?: usb.openDevice(dev)?.also { open[name] = it } ?: continue
            val item = JSObject()
            item.put("fd", conn.fileDescriptor)
            item.put("vendorId", dev.vendorId)
            item.put("productId", dev.productId)
            item.put("serial", conn.serial ?: name)
            list.put(item)
        }
        val ret = JSObject()
        ret.put("devices", list)
        invoke.resolve(ret)
    }

    private fun requestPermission(dev: UsbDevice) {
        val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
        val intent = Intent(ACTION_USB_PERMISSION).setPackage(activity.packageName)
        usb.requestPermission(dev, PendingIntent.getBroadcast(activity, 0, intent, flags))
    }

    companion object {
        const val DJI_VENDOR_ID = 0x2ca3

        /** Receivers the app can drive; keep in sync with res/xml/device_filter.xml. */
        val RECEIVER_PRODUCT_IDS = setOf(0x4011, 0x4015, 0x4115)
        const val ACTION_USB_PERMISSION = "org.djimic.control.USB_PERMISSION"
    }
}
