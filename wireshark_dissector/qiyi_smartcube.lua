require "./aes"

qiyisc_proto = Proto("qiyisc", "QiYi Smart Cube")

local btatt_service_uuid16_f = Field.new("btatt.service_uuid16")
local btatt_value_f = Field.new("btatt.value")
local hci_h4_direction_f = Field.new("hci_h4.direction")

local OP_CUBE_HELLO = 0x2
local OP_STATE_CHANGE = 0x3
local OP_SYNC_CONFIRMATION = 0x4
local opcode_name_map = {
	[OP_CUBE_HELLO] = "Cube Hello", [OP_STATE_CHANGE] = "State Change",
	[OP_SYNC_CONFIRMATION] = "Sync Confirmation",
}

local decbytes_F = ProtoField.bytes("qiyisc.decbytes", "Decrypted Payload")
local opcode_F = ProtoField.uint16("qiyisc.opcode", "Opcode", base.HEX, opcode_name_map)
local length_F = ProtoField.uint8("qiyisc.length", "Length (excl. pad)")
local crc_F = ProtoField.uint16("qiyisc.crc", "Checksum", base.HEX)
local a2c_kind_F = ProtoField.string("qiyisc.a2c_kind", "a2c Message Type")
local ack_of_F = ProtoField.framenum("qiyisc.ack_of", "ACKed Message")
local ackhead_F = ProtoField.bytes("qiyisc.ackhead", "Bytes 3-7 of ACKed Message")
local cubestate_F = ProtoField.bytes("qiyisc.cubestate", "Cube State")
-- State Change
local current_turn_F = ProtoField.bytes("qiyisc.current_turn", "Current Turn")
local previous_turns_F = ProtoField.bytes("qiyisc.previous_turns", "Previous Turns")

qiyisc_proto.fields = {
	decbytes_F, opcode_F, length_F, crc_F, a2c_kind_F, ack_of_F, cubestate_F,
	current_turn_F,previous_turns_F, ackhead_F
}

local ackheads = {}

function qiyisc_proto.dissector(buffer, pinfo, tree)
	local btatt_service_uuid16 = btatt_service_uuid16_f()
	local btatt_value = btatt_value_f()
	local hci_h4_direction = hci_h4_direction_f()

	if btatt_service_uuid16 == nil or btatt_value == nil then
		return
	end

	if btatt_service_uuid16() ~= 0xfff0 then
		return
	end

	local subtree = tree:add(qiyisc_proto, buffer(), qiyisc_proto.description)

	local key = {87, 177, 249, 171, 205, 90, 232, 167, 156, 185, 140, 231, 87, 140, 81, 8}
	local decstring = ciphermode.decryptString(key, btatt_value():raw(), ciphermode.decryptECB)
	local decbuf = ByteArray.new(decstring, true):tvb("Decrypted")
	subtree:add(decbytes_F, decbuf())

	local lenR = decbuf(1, 1)
	subtree:add(length_F, lenR)
	local msglen = lenR:le_uint()

	subtree:add_le(crc_F, decbuf(msglen - 2, 2))

	local is_c2a = hci_h4_direction() == 1
	local is_a2c = not is_c2a

	if is_c2a then
		local opcodeR = decbuf(2, 2)
		subtree:add_le(opcode_F, opcodeR)
		local opcode = opcodeR:le_uint()
		if not pinfo.visited then
			local ackhd = decbuf:bytes(2, 5):raw()
			ackheads[ackhd] = pinfo.number
		end

		pinfo.cols.info = opcode_name_map[opcode] .. " (c->a)"

		if opcode == OP_CUBE_HELLO then
			subtree:add(cubestate_F, decbuf(7, 27))
		elseif opcode == OP_STATE_CHANGE then
			subtree:add(current_turn_F, decbuf(4, 3))
			subtree:add(cubestate_F, decbuf(7, 27))
			subtree:add(previous_turns_F, decbuf(36, 56))
		elseif opcode == OP_SYNC_CONFIRMATION then
			subtree:add(cubestate_F, decbuf(7, 27))
		end
	end

	local guessed_a2c_kind = false

	local a2c_kind = "Unknown"
	if is_a2c and msglen == 9 then
		if guessed_a2c_kind then error("conflicting a2c_kind guess") end
		guessed_a2c_kind = true

		a2c_kind = "ACK"

		local ackhd = decbuf:bytes(2, 5):raw()
		local ackofframe = ackheads[ackhd]

		if ackofframe == nil then
			error("can't find frame that this ACK is of")
		end

		subtree:add(ackhead_F, decbuf(2, 5))
		subtree:add(ack_of_F, ackofframe)
	end
	if is_a2c and msglen == 21 then
		if guessed_a2c_kind then error("conflicting a2c_kind guess") end
		guessed_a2c_kind = true

		a2c_kind = "App Hello"
	end
	if is_a2c and msglen == 38 then
		if guessed_a2c_kind then error("conflicting a2c_kind guess") end
		guessed_a2c_kind = true

		a2c_kind = "Sync State"
		subtree:add(cubestate_F, decbuf(7, 27))
	end
	if is_a2c then
		subtree:add(a2c_kind_F, a2c_kind)
		pinfo.cols.info = a2c_kind .. " (a->c)"
	end
end

register_postdissector(qiyisc_proto)
